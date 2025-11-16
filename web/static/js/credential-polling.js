(() => {
  const statusRoot = document.querySelector('[data-credential-status]');
  if (!statusRoot) {
    return;
  }

  const cidPresent = statusRoot.getAttribute('data-cid-present') === 'true';
  const pollUrl = statusRoot.getAttribute('data-poll-url');

  // Do nothing if credential already issued or we do not have a poll URL
  if (cidPresent || !pollUrl) {
    return;
  }

  const spinner = statusRoot.querySelector('[data-role="spinner"]');
  const statusText = statusRoot.querySelector('[data-role="status-text"]');
  const pollInfo = statusRoot.querySelector('[data-role="poll-info"]');
  let toastRoot = document.querySelector('[data-role="toast-root"]');

  if (!toastRoot) {
    toastRoot = document.createElement('div');
    toastRoot.setAttribute('data-role', 'toast-root');
    document.body.appendChild(toastRoot);
  }
  toastRoot.classList.add('toast-root');

  const toastState = {
    lastMessage: '',
    lastShownAt: 0,
  };

  function showToast(message, tone) {
    if (!message || !toastRoot) {
      return;
    }

    const now = Date.now();
    if (
      toastState.lastMessage === message &&
      now - toastState.lastShownAt < 4000
    ) {
      return;
    }

    toastState.lastMessage = message;
    toastState.lastShownAt = now;

    const toast = document.createElement('div');
    toast.className = [
      'toast',
      tone === 'error' ? 'toast-error' : '',
      tone === 'success' ? 'toast-success' : '',
    ]
      .filter(Boolean)
      .join(' ');
    toast.textContent = message;
    toast.setAttribute('role', 'status');
    toast.setAttribute('aria-live', 'polite');

    toastRoot.appendChild(toast);

    requestAnimationFrame(() => {
      toast.classList.add('visible');
    });

    setTimeout(() => {
      toast.classList.remove('visible');
      setTimeout(() => {
        if (toast.parentNode) {
          toast.parentNode.removeChild(toast);
        }
      }, 250);
    }, 4500);
  }

  let pollCount = 0;
  let pollInterval = 2000;
  const maxPolls = Number(statusRoot.getAttribute('data-max-polls')) || 150;
  let hasTimedOut = false;

  function updatePollingInterval() {
    if (pollCount > 15) {
      pollInterval = 10000;
    } else if (pollCount > 5) {
      pollInterval = 5000;
    } else {
      pollInterval = 2000;
    }
  }

  function handleTimeout() {
    hasTimedOut = true;
    if (spinner) {
      spinner.classList.add('is-hidden');
    }
    if (statusText) {
      statusText.textContent =
        'Polling timed out. Please refresh the page manually.';
    }
    if (pollInfo) {
      pollInfo.textContent = '';
      const refreshButton = document.createElement('button');
      refreshButton.type = 'button';
      refreshButton.className = 'status-refresh-button';
      refreshButton.textContent = 'Refresh Page';
      refreshButton.addEventListener('click', () => {
        window.location.reload();
      });
      pollInfo.appendChild(refreshButton);
    }
  }

  function scheduleNextPoll() {
    pollCount += 1;
    updatePollingInterval();

    if (pollCount >= maxPolls) {
      handleTimeout();
      return;
    }

    window.setTimeout(pollCredentialStatus, pollInterval);
  }

  function handleSuccess() {
    if (spinner) {
      spinner.classList.add('is-hidden');
    }
    if (statusText) {
      statusText.textContent = 'Credential issued! ✓';
    }
    if (pollInfo) {
      pollInfo.textContent = 'Reloading to show your credential...';
    }
    showToast('Credential issued! Reloading...', 'success');
    window.setTimeout(() => {
      window.location.reload();
    }, 1000);
  }

  async function pollCredentialStatus() {
    if (hasTimedOut) {
      return;
    }

    if (pollInfo) {
      const attempts = pollCount + 1;
      pollInfo.textContent = `Checked ${attempts} time${
        attempts === 1 ? '' : 's'
      }... still waiting for wallet scan.`;
    }

    try {
      const response = await fetch(pollUrl, {
        method: 'GET',
        cache: 'no-store',
      });

      if (response.status === 200) {
        const data = await response
          .json()
          .catch(() => ({ status: 'ready' }));

        if (data && data.status === 'ready') {
          handleSuccess();
          return;
        }

        // Fallback when response OK but not ready yet
        if (pollInfo) {
          const attempts = pollCount + 1;
          pollInfo.textContent = `Checked ${attempts} time${
            attempts === 1 ? '' : 's'
          }... still waiting for wallet scan.`;
        }
      } else if (response.status === 202) {
        // Expected pending state, nothing special to do
      } else {
        const text =
          (await response.text()) ||
          'Unexpected response while checking credential status.';
        showToast(text, 'error');

        if (response.status >= 400 && response.status < 500) {
          hasTimedOut = true;

          if (spinner) {
            spinner.classList.add('is-hidden');
          }
          if (statusText) {
            statusText.textContent = text;
          }
          if (pollInfo) {
            pollInfo.textContent = '';
          }
          return;
        }
      }
    } catch (error) {
      console.error('Credential polling error:', error);
      showToast(
        "Temporary issue checking credential. We'll keep trying in the background.",
        'error',
      );
    }

    scheduleNextPoll();
  }

  // Kick things off
  pollCredentialStatus();
})();
