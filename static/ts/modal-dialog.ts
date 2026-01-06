/**
 * A custom element for modal dialogs that wraps content with
 * standard modal structure (overlay, container, header, close button).
 *
 * Usage:
 *   <modal-dialog title="My Title" close-target="modal-container-id">
 *     <form>...</form>
 *   </modal-dialog>
 *
 * Attributes:
 *   - title: The modal title (required)
 *   - close-target: ID of the element to clear when closing (required)
 *   - max-width: Tailwind max-width class (default: "max-w-md")
 */
class ModalDialog extends HTMLElement {
    connectedCallback() {
        const title = this.getAttribute('title') || '';
        const closeTarget = this.getAttribute('close-target') || '';
        const maxWidth = this.getAttribute('max-width') || 'max-w-md';

        // Get the original content
        const content = this.innerHTML;

        // Build the modal structure
        this.innerHTML = `
            <div class="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50" id="modal-overlay">
                <div class="bg-white dark:bg-gray-800 rounded-lg shadow-xl p-6 ${maxWidth} w-full mx-4">
                    <div class="flex justify-between items-center mb-4">
                        <h2 class="text-2xl font-bold">${this.escapeHtml(title)}</h2>
                        <button
                            type="button"
                            class="modal-close-btn text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-200">
                            <svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"></path>
                            </svg>
                        </button>
                    </div>
                    <div class="modal-content">
                        ${content}
                    </div>
                </div>
            </div>
        `;

        // Set up close button handler
        const closeBtn = this.querySelector('.modal-close-btn');
        if (closeBtn && closeTarget) {
            closeBtn.addEventListener('click', () => {
                const target = document.getElementById(closeTarget);
                if (target) {
                    target.innerHTML = '';
                }
            });
        }

        // Close on overlay click
        const overlay = this.querySelector('#modal-overlay');
        if (overlay && closeTarget) {
            overlay.addEventListener('click', (e) => {
                if (e.target === overlay) {
                    const target = document.getElementById(closeTarget);
                    if (target) {
                        target.innerHTML = '';
                    }
                }
            });
        }
    }

    private escapeHtml(text: string): string {
        const div = document.createElement('div');
        div.textContent = text;
        return div.innerHTML;
    }
}

customElements.define('modal-dialog', ModalDialog);
