import { actions } from './actions.js';
import DOM from './dom.js';
import * as s from './states.js';
import * as crop from './crop.js';

function arrayEquals(a, b) {
    if (a.length !== b.length) return false;
    for (let i = 0; i < a.length; ++i) {
        if (a[i] !== b[i]) return false;
    }
    return true;
}

function formatRecipientList(list) {
    if (list.length == 0) {
        return '';
    } else if (list.length == 1) {
        return list[0];
    } else if (list.length <= 4) {
        return list.slice(0, -1).join(', ') + ' og ' + list.slice(-1)[0];
    } else {
        return list.length + ' mottakere';
    }
}


function renderPreview(prev, next) {
    if (next.previewUrl != prev.previewUrl) {
        for (let img of DOM.previewImages) {
            img.src = next.previewUrl;
        }
    }

    if (next.showPreview != prev.showPreview) {
        DOM.preview.style.display = next.showPreview ? "block" : "none";
    }

    if ((next.showPreview && !prev.showPreview) || (next.previewUrl != prev.previewUrl)) {
        DOM.preview.scrollIntoView();
    }
}

function renderUpload(prev, next) {
    if (next.uploadPhase != prev.uploadPhase) {
        console.log("Updating for upload phase", next.uploadPhase);
        DOM.uploader.statusUploading.style.display = (next.uploadPhase == s.UPLOAD_PHASE_IN_PROGRESS ? 'block' : 'none');
        DOM.uploader.statusUploaded.style.display = (next.uploadPhase == s.UPLOAD_PHASE_FINISHED ? 'block' : 'none');
    }

    if (next.uploadError != prev.uploadError) {
        if (next.uploadError) {
            DOM.uploader.errorMessage.textContent = next.uploadError.hint;
        }

        DOM.uploader.uploadError.style.display = next.uploadError ? 'block' : 'none';
    }

    if (next.uploadResult !== prev.uploadResult) {
        DOM.details.detailsSubmission.style.display =
            next.uploadResult == s.UPLOAD_STATE_SUCCESS ? "block" : "none";
    }
}

let submitHandlerChanged = false;
let submitHandlerAttached = false;

function submitHandler(ev) {
    ev.preventDefault();
    ev.stopPropagation();

    if (submitHandlerChanged) {
        actions.submitDetails();
    } else {
        actions.reset();
    }
}

function renderMetadataForm(prev, next) {
    submitHandlerChanged = next.changed.any;
    if (!submitHandlerAttached) {
        DOM.details.form.addEventListener('submit', submitHandler);
        submitHandlerAttached = true;
    }

    if (next.formEnabled != prev.formEnabled) {
        const disabledString = next.formEnabled ? "" : "disabled";
        for (let element of DOM.details.form.elements) {
            element.disabled = disabledString;
        }
    }

    if (next.comment != prev.comment) {
        DOM.details.comment.value = next.comment;
    }

    const changedUpdated =
        next.changed.any != prev.changed.any ||
        next.changed.crop != prev.changed.crop ||
        next.changed.recipients != prev.changed.recipients ||
        !arrayEquals(next.changed.newRecipients, prev.changed.newRecipients) ||
        !arrayEquals(next.changed.removedRecipients, prev.changed.removedRecipients);
    const commentUpdated = next.comment != prev.comment;
    const sendEmailUpdated = next.sendEmail != prev.sendEmail;
    if (changedUpdated || commentUpdated || sendEmailUpdated) {
        let summary = "", action;
        if (!next.changed.any) {
            summary = "Ingen endringer.";
            action = "Lukk bildet";
        } else {
            if (next.changed.crop) {
                summary = "Beskjæringen er oppdatert. ";
            }
            if (next.changed.comment) {
                summary += "Kommentaren er endret. ";
            }
            if (next.changed.recipients) {
                if (next.changed.newRecipients.length > 0) {
                    summary += `Du har lagt til ${formatRecipientList(next.changed.newRecipients)}`;
                }
                if (next.changed.removedRecipients.length > 0) {
                    if (next.changed.newRecipients.length > 0) {
                        summary += " og";
                    } else {
                        summary += "Du har";
                    }
                    summary += " fjernet " + formatRecipientList(next.changed.removedRecipients);
                }
                summary += "."
            }

            action = "💾 Lagre";
            if (next.changed.newRecipients.length > 0 && next.sendEmail) {
                action += " og 📨 send epost"
            }
        }

        DOM.details.summary.textContent = summary;
        DOM.details.submit.textContent = action;
    }

    if (next.saveDetailsState != prev.saveDetailsState) {
        let msg = "";
        switch (next.saveDetailsState) {
            case s.SAVE_DETAILS_INITIAL: msg = ""; break;
            case s.SAVE_DETAILS_IN_PROGRESS: msg = "Lagrer…"; break;
            case s.SAVE_DETAILS_FAILED: msg = "😕 Noe skar seg. " + next.saveDetailsError.hint; break;
        }

        DOM.details.status.textContent = msg;
    }
}

function renderEmailForm(prev, next) {
    if ((next.changed.newRecipients.length > 0) != (prev.changed.newRecipients.length > 0)) {
        const action = next.changed.newRecipients.length > 0 ? 'add' : 'remove';
        DOM.email.container.classList[action]('show');
    }

    if (!arrayEquals(next.changed.newRecipients, prev.changed.newRecipients)) {
        DOM.email.recipients.textContent = formatRecipientList(next.changed.newRecipients);
    }

    if (next.seriesUrl != prev.seriesUrl) {
        DOM.email.link.href = DOM.uploader.url.href = next.seriesUrl;
        DOM.email.link.textContent = DOM.uploader.url.textContent = next.seriesUrl;
    }

    if (next.sendEmail != prev.sendEmail) {
        const action = next.sendEmail ? 'add' : 'remove';
        DOM.email.emailDetails.classList[action]('show');

        const disabled = !next.sendEmail;
        DOM.email.title.disabled = disabled;
        DOM.email.messageInput.disabled = disabled;
    }

    if (next.emailMessage != prev.emailMessage) {
        DOM.email.messagePreview.textContent = next.emailMessage;
    }
}

let beforeUnloadHandlerChanged = false;
let beforeUnloadHandlerAttached = false;

function beforeUnloadHandler(ev) {
    if (beforeUnloadHandlerChanged) {
        ev.preventDefault();
        ev.returnValue = '';
    }
}

function updateBeforeUnloadHandler(prev, next) {
    beforeUnloadHandlerChanged = next.changed.any;

    if (!beforeUnloadHandlerAttached) {
        window.addEventListener('beforeunload', beforeUnloadHandler);
        beforeUnloadHandlerAttached = true;
    }
}

function render(prev, next) {
    if (next.phase != prev.phase) {
        DOM.phase.initial.style.display = (next.phase == s.PHASE_INITIAL ? 'block' : 'none');
        DOM.phase.preview.style.display = (next.phase == s.PHASE_PREVIEW ? 'block' : 'none');
        DOM.phase.details.style.display = (next.phase == s.PHASE_DETAILS ? 'block' : 'none');
    }

    renderPreview(prev, next);
    renderUpload(prev, next);
    renderMetadataForm(prev, next);
    crop.render(prev, next);
    renderEmailForm(prev, next);

    updateBeforeUnloadHandler(prev, next);
}

export default render;
