// TODO: T044 - Implement QR code scanning functionality
// This will use html5-qrcode library or similar for camera-based QR scanning

/**
 * Initialize QR code scanner
 * Requires html5-qrcode library to be included in the HTML
 */
function initQRScanner() {
    // TODO: Implement actual scanner initialization
    // Example structure:
    // const html5QrCode = new Html5Qrcode("qr-reader");
    // html5QrCode.start(
    //     { facingMode: "environment" },
    //     { fps: 10, qrbox: 250 },
    //     onScanSuccess,
    //     onScanError
    // );

    console.log('QR Scanner initialization - TODO');
}

/**
 * Handle successful QR code scan
 * @param {string} decodedText - The decoded QR code content
 */
function onScanSuccess(decodedText) {
    console.log('QR Code scanned:', decodedText);

    // TODO: Send decoded content to backend for verification
    // fetch('/verify/scan', {
    //     method: 'POST',
    //     headers: { 'Content-Type': 'application/json' },
    //     body: JSON.stringify({ qr_payload: decodedText })
    // })
    // .then(response => response.json())
    // .then(data => displayVerificationResult(data))
    // .catch(error => console.error('Verification failed:', error));
}

/**
 * Handle QR scan errors
 * @param {string} errorMessage - Error message from scanner
 */
function onScanError(errorMessage) {
    // Ignore minor errors during scanning
    if (errorMessage.includes('NotFoundException')) {
        return;
    }
    console.warn('QR Scan error:', errorMessage);
}

/**
 * Display verification result to user
 * @param {Object} result - Verification result from backend
 */
function displayVerificationResult(result) {
    const resultDiv = document.getElementById('verification-result');
    const detailsDiv = document.getElementById('result-details');
    const statusElement = document.getElementById('result-status');

    resultDiv.classList.remove('hidden');

    if (result.is_valid) {
        statusElement.textContent = '✅ 驗證成功';
        statusElement.style.color = 'var(--color-success)';

        detailsDiv.innerHTML = `
            <p><strong>頻道:</strong> ${result.issuer_name}</p>
            <p><strong>會員等級:</strong> ${result.member_level || '基本會員'}</p>
            <p><strong>有效期限:</strong> ${result.expiry_date}</p>
        `;
    } else {
        statusElement.textContent = '❌ 驗證失敗';
        statusElement.style.color = 'var(--color-error)';

        detailsDiv.innerHTML = `
            <p>${result.error_message || '無效的會員卡'}</p>
        `;
    }
}

// Initialize scanner when page loads
if (document.getElementById('qr-reader')) {
    document.addEventListener('DOMContentLoaded', initQRScanner);
}
