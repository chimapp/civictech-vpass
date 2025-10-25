// Card deletion with confirmation dialog
document.addEventListener('DOMContentLoaded', () => {
  const deleteButton = document.querySelector('[data-delete-card]');

  if (!deleteButton) {
    return;
  }

  deleteButton.addEventListener('click', async (e) => {
    const cardId = e.target.dataset.deleteCard;

    // Show confirmation dialog
    const confirmed = confirm(
      'Are you sure you want to delete this card?\n\n' +
      'This action cannot be undone. The card will be permanently removed from your collection.'
    );

    if (!confirmed) {
      return;
    }

    // Disable button to prevent double-clicks
    deleteButton.disabled = true;
    deleteButton.textContent = 'Deleting...';

    try {
      const response = await fetch(`/cards/${cardId}`, {
        method: 'DELETE',
        headers: {
          'Content-Type': 'application/json',
        },
      });

      if (response.ok) {
        // Redirect will be handled by the server response
        window.location.href = '/cards/my-cards?deleted=true';
      } else {
        // Handle error
        alert('Failed to delete card. Please try again.');
        deleteButton.disabled = false;
        deleteButton.textContent = 'Delete Card';
      }
    } catch (error) {
      console.error('Error deleting card:', error);
      alert('An error occurred while deleting the card. Please try again.');
      deleteButton.disabled = false;
      deleteButton.textContent = 'Delete Card';
    }
  });
});
