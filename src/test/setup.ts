import '@testing-library/jest-dom/vitest';

// jsdom does not implement the modal <dialog> API.
if (!('showModal' in HTMLDialogElement.prototype)) {
  Object.assign(HTMLDialogElement.prototype, {
    showModal(this: HTMLDialogElement) {
      this.setAttribute('open', '');
    },
    close(this: HTMLDialogElement) {
      this.removeAttribute('open');
    },
  });
}
