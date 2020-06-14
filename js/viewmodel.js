import * as s from './states.js';

function changedCrop(crop) {
    return crop.start !== crop.savedStart || crop.end !== crop.savedEnd;
}

export default function (state) {
    const changedCropHorizontal = changedCrop(state.cropHorizontal);
    const changedCropVertical = changedCrop(state.cropVertical);

    const pickingImage = state.phase === s.PHASE_PREVIEW;
    const uploadingImage = state.uploadPhase === s.UPLOAD_PHASE_IN_PROGRESS;
    const imageUploadFailed = state.uploadResult === s.UPLOAD_STATE_FAILURE;

    const newRecipients = [];
    const removedRecipients = [];

    const prevRec = state.savedRecipients, nextRec = state.recipients;
    for (let pi = 0, ni = 0; pi < prevRec.length || ni < nextRec.length; ) {
        let p = prevRec[pi], n = nextRec[ni];
        if (p < n || n == undefined) {
            removedRecipients.push(p);
            pi++;
        } else if (p > n || p == undefined) {
            newRecipients.push(n);
            ni++;
        } else {
            pi++;
            ni++;
        }
    }

    const changed = {
        image: pickingImage || uploadingImage || imageUploadFailed,
        crop: changedCropHorizontal || changedCropVertical,
        recipients: !!(newRecipients.length || removedRecipients.length),
        newRecipients,
        removedRecipients,
    };

    changed.any = changed.image || changed.crop || changed.recipients;

    return {
        ...state,
        showPreview: state.phase >= s.PHASE_PREVIEW,
        formEnabled:
            (state.saveDetailsState != s.SAVE_DETAILS_IN_PROGRESS) &&
            (state.loadDetailsState == s.LOAD_DETAILS_READY),
        changed,
    }
};
