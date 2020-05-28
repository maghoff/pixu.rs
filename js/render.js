import DOM from './dom.js';
import * as s from './states.js';
import * as crop from './crop.js';

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

function renderMetadataForm(prev, next) {
    if (next.formEnabled != prev.formEnabled) {
        const disabledString = next.formEnabled ? "" : "disabled";
        for (let element of DOM.details.form.elements) {
            element.disabled = disabledString;
        }
    }

    if (next.saveDetailsState != prev.saveDetailsState) {
        let msg;
        switch (next.saveDetailsState) {
            case s.SAVE_DETAILS_INITIAL: msg = "Er alt klart da?"; break;
            case s.SAVE_DETAILS_IN_PROGRESS: msg = "Deler…"; break;
            case s.SAVE_DETAILS_FAILED: msg = "😕 Noe skar seg. " + next.saveDetailsError.hint; break;
        }

        DOM.details.status.textContent = msg;
    }
}

function renderEmailForm(prev, next) {
    if (next.pixurUrl != prev.pixurUrl) {
        DOM.email.link.href = DOM.uploader.pixurUrl.href = next.pixurUrl;
        DOM.email.link.textContent = DOM.uploader.pixurUrl.textContent = next.pixurUrl;
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
}

export default render;
