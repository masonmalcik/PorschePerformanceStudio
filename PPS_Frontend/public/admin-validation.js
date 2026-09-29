const errorMessage = 'This Field is Required';

function errorIdFor(field) {
  if (!field.id) field.id = `admin-field-${Math.random().toString(36).slice(2)}`;
  return `${field.id}-error`;
}

function showFieldError(field) {
  const wrapper = field.closest('label');
  if (!wrapper) return;

  wrapper.classList.add('form-field', 'invalid');
  field.setAttribute('aria-invalid', 'true');
  const errorId = errorIdFor(field);
  field.setAttribute('aria-describedby', errorId);

  let message = wrapper.querySelector('.field-error');
  if (!message) {
    message = document.createElement('small');
    message.className = 'field-error';
    wrapper.append(message);
  }
  message.id = errorId;
  message.textContent = errorMessage;
}

function clearFieldError(field) {
  const wrapper = field.closest('label');
  if (!wrapper) return;
  wrapper.classList.remove('invalid');
  field.removeAttribute('aria-invalid');
  field.removeAttribute('aria-describedby');
  wrapper.querySelector('.field-error')?.remove();
}

document.addEventListener('invalid', (event) => {
  const field = event.target;
  if (!(field instanceof HTMLInputElement || field instanceof HTMLTextAreaElement || field instanceof HTMLSelectElement)) return;
  if (!field.closest('.admin-form')) return;
  event.preventDefault();
  showFieldError(field);
}, true);

document.addEventListener('input', (event) => {
  const field = event.target;
  if (!(field instanceof HTMLInputElement || field instanceof HTMLTextAreaElement || field instanceof HTMLSelectElement)) return;
  if (!field.closest('.admin-form')) return;
  if (field.closest('label')?.classList.contains('invalid')) clearFieldError(field);
});

document.addEventListener('change', (event) => {
  const field = event.target;
  if (!(field instanceof HTMLInputElement || field instanceof HTMLTextAreaElement || field instanceof HTMLSelectElement)) return;
  if (!field.closest('.admin-form')) return;
  if (field.closest('label')?.classList.contains('invalid')) clearFieldError(field);
});
