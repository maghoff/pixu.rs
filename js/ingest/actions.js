import * as s from './states.js';
import DOM from './dom.js';
import { state, setState, updateState, initialState } from './store.js';

// Safe aspect ratios:

// From the author's tall, narrow phone:
const SAFE_PORTRAIT_ASPECT = 414 / 837;

// From a browser window on a small laptop:
const SAFE_LANDSCAPE_ASPECT = 699 / 1280;

function gatherDetails() {
    const details = {
        metadata: {
            recipients: state.recipients.slice(0),
            crop_left: state.cropHorizontal.start,
            crop_right: state.cropHorizontal.end,
            crop_top: state.cropVertical.start,
            crop_bottom: state.cropVertical.end,
        },
        send_email: DOM.email.sendEmail.checked ? {
            title: DOM.email.title.value,
            message: state.emailMessage,
        } : null,
    };

    return details;
}

function setDetails(details) {
    const options = DOM.details.recipients.options;
    for (let option of options) {
        option.selected = details.recipients.indexOf(option.value) != -1;
    }
}

export const actions = {
    selectFile: function (file) {
        if (file) {
            const img = new Image();

            // It is glitch-free to initialize the crop values async. The image
            // will load immediately and definitely before the user proceeds to
            // the phase where the crop controls are visible.
            img.onload = () => {
                const imageAspect = img.naturalWidth / img.naturalHeight;

                const cropHalfWidth = Math.min(SAFE_PORTRAIT_ASPECT / imageAspect, 1) / 2;
                const cropHalfHeight = Math.min(SAFE_LANDSCAPE_ASPECT * imageAspect, 1) / 2;

                updateState({
                    cropHorizontal: {
                        start: 0.5 - cropHalfWidth,
                        end: 0.5 + cropHalfWidth,
                        savedStart: null,
                        savedEnd: null,
                    },
                    cropVertical: {
                        start: 0.5 - cropHalfHeight,
                        end: 0.5 + cropHalfHeight,
                        savedStart: null,
                        savedEnd: null,
                    }
                })
            };
            img.src = window.URL.createObjectURL(file);
        }

        updateState({
            phase: file ? s.PHASE_PREVIEW : s.PHASE_INITIAL,
            file: file || null,
            previewUrl: file ? window.URL.createObjectURL(file) : "",
            savedRecipients: [],
            recipients: [],
            cropHorizontal: {
                start: 0.5,
                end: 0.5,
                savedStart: null,
                savedEnd: null,
            },
            cropVertical: {
                start: 0.5,
                end: 0.5,
                savedStart: null,
                savedEnd: null,
            }
        });
    },
    reset: function () {
        setState(initialState);
    },
    upload: function (file) {
        fetch('img/', {
            method: 'POST',
            body: file,
            credentials: 'same-origin',
            redirect: 'follow',
        })
            .catch(function (err) {
                // Low level error situation, such as network error
                throw {
                    err: err,
                    hint: s.ERROR_CHECK_CONNECTIVITY,
                };
            })
            .then(function (res) {
                try {
                    if (!res.ok) {
                        throw "Unexpected status code: " + res.status + " " + res.statusText;
                    }

                    const location = res.headers.get('location');
                    if (!location) {
                        throw "Missing Location header in server response";
                    }

                    actions.uploadFinished(location);
                }
                catch (err) {
                    // Unexpected error
                    throw {
                        err: err,
                        hint: s.ERROR_TRY_AGAIN,
                    };
                }
            })
            .catch(function (err) {
                updateState({
                    uploadPhase: s.UPLOAD_PHASE_FINISHED,
                    uploadResult: s.UPLOAD_STATE_FAILURE,
                    uploadError: err,
                });
            });

        updateState({
            phase: s.PHASE_DETAILS,
            uploadPhase: s.UPLOAD_PHASE_IN_PROGRESS,
            uploadResult: null,
            uploadError: null,
            pixurUrl: null,
            loadDetailsState: s.LOAD_DETAILS_READY,
        });
    },
    uploadFinished: function (location) {
        updateState({
            uploadPhase: s.UPLOAD_PHASE_FINISHED,
            uploadResult: s.UPLOAD_STATE_SUCCESS,
            pixurUrl: location,
        });
    },
    submitDetails: function () {
        let details = gatherDetails();

        fetch(state.pixurUrl + "/meta", {
            method: 'POST',
            body: JSON.stringify(details),
            headers: {
                'Content-Type': 'application/json'
            },
            credentials: 'same-origin',
            redirect: 'follow',
        })
            .catch(function (err) {
                // Low level error situation, such as network error
                throw {
                    err: err,
                    hint: s.ERROR_CHECK_CONNECTIVITY,
                };
            })
            .then(function (res) {
                try {
                    if (!res.ok) {
                        throw "Unexpected status code: " + res.status + " " + res.statusText;
                    }
                    alert("Lagret ✔");
                    updateState({
                        saveDetailsState: s.SAVE_DETAILS_SUCCEEDED,
                        savedRecipients: state.recipients,
                        cropHorizontal: {
                            start: state.cropHorizontal.start,
                            end: state.cropHorizontal.end,
                            savedStart: state.cropHorizontal.start,
                            savedEnd: state.cropHorizontal.end,
                        },
                        cropVertical: {
                            start: state.cropVertical.start,
                            end: state.cropVertical.end,
                            savedStart: state.cropVertical.start,
                            savedEnd: state.cropVertical.end,
                        },
                    });
                }
                catch (err) {
                    // Unexpected error
                    throw {
                        err: err,
                        hint: s.ERROR_TRY_AGAIN,
                    };
                }
            })
            .catch(function (err) {
                updateState({
                    saveDetailsState: s.SAVE_DETAILS_FAILED,
                    saveDetailsError: err,
                });
            });

        updateState({
            saveDetailsState: s.SAVE_DETAILS_IN_PROGRESS,
        });
    },
    selectExistingImage: function (pixurUrl, thumb, hr) {
        fetch(pixurUrl + "/meta", {
            credentials: 'same-origin',
            redirect: 'follow',
        })
            .catch(function (err) {
                // Low level error situation, such as network error
                throw {
                    err: err,
                    hint: s.ERROR_CHECK_CONNECTIVITY,
                };
            })
            .then(function (res) {
                try {
                    if (!res.ok) {
                        throw "Unexpected status code: " + res.status + " " + res.statusText;
                    }
                    return res.json();
                }
                catch (err) {
                    // Unexpected error
                    throw {
                        err: err,
                        hint: s.ERROR_TRY_AGAIN,
                    };
                }
            })
            .then(function (metadata) {
                try {
                    if (state.pixurUrl != pixurUrl) return;

                    // Abstraction leak, updating DOM outside of render:
                    setDetails(metadata);

                    updateState({
                        loadDetailsState: s.LOAD_DETAILS_READY,
                        savedRecipients: metadata.recipients,
                        recipients: metadata.recipients,
                        cropHorizontal: {
                            start: metadata.crop_left,
                            end: metadata.crop_right,
                            savedStart: metadata.crop_left,
                            savedEnd: metadata.crop_right,
                        },
                        cropVertical: {
                            start: metadata.crop_top,
                            end: metadata.crop_bottom,
                            savedStart: metadata.crop_top,
                            savedEnd: metadata.crop_bottom,
                        }
                    });
                }
                catch (err) {
                    // Unexpected error
                    throw {
                        err: err,
                        hint: s.ERROR_TRY_AGAIN,
                    };
                }
            })
            .catch(function (err) {
                if (state.pixurUrl != pixurUrl) return;
                updateState({
                    loadDetailsState: s.LOAD_DETAILS_FAILED,
                    loadDetailsError: err,
                });
            });

        updateState({
            phase: s.PHASE_DETAILS,
            uploadPhase: s.UPLOAD_PHASE_FINISHED,
            uploadResult: s.UPLOAD_STATE_SUCCESS,
            pixurUrl,
            previewUrl: thumb,
            loadDetailsState: s.LOAD_DETAILS_PENDING,
            saveDetailsState: s.SAVE_DETAILS_INITIAL,
        });

        setTimeout(() => {
            if (state.pixurUrl != pixurUrl) return;
            updateState({ previewUrl: hr });
        }, 0);
    },
};
