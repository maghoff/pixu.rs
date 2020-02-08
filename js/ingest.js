import DOM from './dom.js';
import { actions, updateState, state } from './actions.js';
import crop from './crop.js';

DOM.fileInput.addEventListener('change', function (ev) {
    ev.preventDefault();
    ev.stopPropagation();
    actions.selectFile(DOM.fileInput.files[0]);
});

DOM.uploaderForm.addEventListener('reset', function (ev) {
    actions.reset();
});

DOM.uploaderForm.addEventListener('submit', function (ev) {
    ev.preventDefault();
    ev.stopPropagation();
    actions.upload(state.file);
});

document.getElementById("uploader-form--add-recipient").addEventListener('click', function (ev) {
    const email = prompt("Epostadresse");
    if (email) {
        const opt = document.createElement("option");
        opt.setAttribute("value", email);
        opt.textContent = email;
        opt.setAttribute("selected", "selected");

        const sel = document.querySelector(".uploader-form--recipients");
        sel.appendChild(opt);
        sel.size = sel.options.length;
    }
});

DOM.details.form.addEventListener('submit', function (ev) {
    ev.preventDefault();
    ev.stopPropagation();
    actions.submitDetails();
});

// ## Metadata editor ##
document.querySelector('.thumbnails').addEventListener('click', function (ev) {
    ev.preventDefault();
    ev.stopPropagation();

    let li = ev.target;
    while (li && li.tagName != "LI") {
        li = li.parentNode;
    }

    if (!li) return;

    const pixurUrl = li.querySelector("a").href;
    const thumb = li.querySelector("img").src;
    const hr = li.querySelector("img").getAttribute('data-hr');

    actions.selectExistingImage(pixurUrl, thumb, hr);
});

// ## Email ##
DOM.email.sendEmail.addEventListener('input', function (ev) {
    updateState({ sendEmail: DOM.email.sendEmail.checked });
});

DOM.email.messageInput.addEventListener('input', function (ev) {
    updateState({ emailMessage: DOM.email.messageInput.value });
});

DOM.email.link.addEventListener('click', function (ev) {
    ev.preventDefault();
    ev.stopPropagation();
});


// Handle autofilling by browsers:
actions.selectFile(DOM.fileInput.files[0]);

updateState({
    sendEmail: DOM.email.sendEmail.checked,
    emailMessage: DOM.email.messageInput.value,
});
DOM.email.messagePreview.textContent = state.emailMessage;
