import DOM from './dom.js';
import * as s from './states.js';

function render(prev, next) {
    if (next.phase != prev.phase) {
        DOM.phase.initial.style.display = (next.phase == s.PHASE_INITIAL ? 'block' : 'none');
        DOM.phase.preview.style.display = (next.phase == s.PHASE_PREVIEW ? 'block' : 'none');
        DOM.phase.details.style.display = (next.phase == s.PHASE_DETAILS ? 'block' : 'none');
    }

    // Preview
    if (next.previewUrl != prev.previewUrl) {
        for (let img of DOM.previewImages) {
            img.src = next.previewUrl;
        }
    }

    const oldShowPreview = prev.phase >= s.PHASE_PREVIEW;
    const newShowPreview = next.phase >= s.PHASE_PREVIEW;
    if (newShowPreview != oldShowPreview) {
        DOM.preview.style.display = newShowPreview ? "block" : "none";
    }

    if ((newShowPreview && !oldShowPreview) || (next.previewUrl != prev.previewUrl)) {
        DOM.preview.scrollIntoView();
    }

    // Upload
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

    // Metadata form
    const prevFormEnabled =
        (prev.saveDetailsState != s.SAVE_DETAILS_IN_PROGRESS) &&
        (prev.loadDetailsState == s.LOAD_DETAILS_READY);
    const nextFormEnabled =
        (next.saveDetailsState != s.SAVE_DETAILS_IN_PROGRESS) &&
        (next.loadDetailsState == s.LOAD_DETAILS_READY);

    if (nextFormEnabled != prevFormEnabled) {
        const disabledString = nextFormEnabled ? "" : "disabled";
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

    // Cropping
    function renderCrop(dom, prev, next, startAnchor, endAnchor) {
        if (next.start !== prev.start) {
            dom.start.style[startAnchor] = ((1 - next.start) * 100) + "%";
        }
        if (next.end !== prev.end) {
            dom.end.style[endAnchor] = (next.end * 100) + "%";
        }
        if ((next.start !== prev.start) || (next.end !== prev.end)) {
            dom.middle.style[endAnchor] = ((next.start + next.end) / 2 * 100) + "%";
        }

        const nextStartDrag = next.dragging == "start";
        const prevStartDrag = prev.dragging == "start";
        if (nextStartDrag !== prevStartDrag) {
            const action = nextStartDrag ? "add" : "remove";
            dom.startHandle.classList[action]("cropping--handle__active");
        }

        const nextEndDrag = next.dragging == "end";
        const prevEndDrag = prev.dragging == "end";
        if (nextEndDrag !== prevEndDrag) {
            const action = nextEndDrag ? "add" : "remove";
            dom.endHandle.classList[action]("cropping--handle__active");
        }
    }

    if (nextFormEnabled != prevFormEnabled) {
        const action = nextFormEnabled ? "removeAttribute" : "setAttribute";
        const elements = [
            DOM.crop.horizontal.start,
            DOM.crop.horizontal.startHandle,
            DOM.crop.horizontal.middle,
            DOM.crop.horizontal.middleHandle,
            DOM.crop.horizontal.end,
            DOM.crop.horizontal.endHandle,
            DOM.crop.vertical.start,
            DOM.crop.vertical.startHandle,
            DOM.crop.vertical.middle,
            DOM.crop.vertical.middleHandle,
            DOM.crop.vertical.end,
            DOM.crop.vertical.endHandle,
        ];
        for (let element of elements) {
            element[action]("disabled", "disabled");
        }
    }

    const prevInitialized = prev.loadDetailsState == s.LOAD_DETAILS_READY;
    const nextInitialized = next.loadDetailsState == s.LOAD_DETAILS_READY;

    if (nextInitialized != prevInitialized) {
        const action = nextInitialized ? "remove" : "add";
        DOM.crop.horizontal.root.classList[action]("cropping__uninitialized");
        DOM.crop.vertical.root.classList[action]("cropping__uninitialized");
    }

    renderCrop(DOM.crop.horizontal, prev.cropHorizontal, next.cropHorizontal, "right", "left");
    renderCrop(DOM.crop.vertical, prev.cropVertical, next.cropVertical, "bottom", "top");

    // Email form
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

export default render;
