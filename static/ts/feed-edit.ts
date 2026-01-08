/**
 * Feed edit form functionality
 * - Color picker sync with text input
 * - Form submission with custom frequency handling
 */

function initFeedEditForm(): void {
  const colorPicker = document.getElementById('color') as HTMLInputElement | null;
  const colorText = document.getElementById('color-text') as HTMLInputElement | null;
  const form = document.getElementById('frequency-form') as HTMLFormElement | null;

  if (colorPicker && colorText) {
    // Sync color picker with text input
    colorPicker.addEventListener('input', function() {
      colorText.value = this.value.toUpperCase();
    });

    colorText.addEventListener('input', function() {
      if (/^#[0-9A-Fa-f]{6}$/.test(this.value)) {
        colorPicker.value = this.value;
      }
    });
  }

  if (form) {
    // Handle HTMX form submission to combine radio + custom hours
    form.addEventListener('htmx:configRequest', function(event: Event) {
      const htmxEvent = event as CustomEvent;
      const radios = document.getElementsByName('fetch_frequency') as NodeListOf<HTMLInputElement>;
      const customHours = document.getElementById('custom_hours') as HTMLInputElement | null;

      for (const radio of radios) {
        if (radio.checked && radio.value === 'custom' && customHours) {
          // Replace 'custom' with actual hours value in the request parameters
          htmxEvent.detail.parameters['fetch_frequency'] = customHours.value;
          break;
        }
      }
    });
  }
}

// Initialize on page load
document.addEventListener('DOMContentLoaded', initFeedEditForm);

// Also initialize after HTMX loads (in case the form is loaded dynamically)
document.body.addEventListener('htmx:afterSettle', initFeedEditForm);
