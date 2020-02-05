import * as s from './states.js';
import DOM from './dom.js';
import render from './render.js';

const initialState = {
    phase: s.PHASE_INITIAL,
    uploadPhase: s.UPLOAD_PHASE_INACTIVE,
    loadDetailsState: s.LOAD_DETAILS_READY,
    saveDetailsState: s.SAVE_DETAILS_INITIAL,
    previewUrl: "",
    sendEmail: DOM.email.sendEmail.defaultChecked,
    emailMessage: DOM.email.messageInput.defaultValue,
};
export let state = initialState;

function setState(newState) {
    console.log(newState);
    render(state, newState);
    state = newState;
}

export function updateState(delta) {
    const newState = { ...state, ...delta };
    setState(newState);
}

function gatherDetails() {
    const details = {
        metadata: {
            recipients: [],
        },
        send_email: DOM.email.sendEmail.checked ? {
            title: DOM.email.title.value,
            message: state.emailMessage,
        } : null,
    };

    const s = document.querySelector(".uploader-form--recipients").selectedOptions;
    for (let i = 0; i < s.length; ++i) {
        details.metadata.recipients.push(s[i].value);
    }

    return details;
}

function setDetails(details) {
    const options = document.querySelector(".uploader-form--recipients").options;
    for (let option of options) {
        option.selected = details.recipients.indexOf(option.value) != -1;
    }
}

export const actions = {
    selectFile: function (file) {
        updateState({
            phase: file ? s.PHASE_PREVIEW : s.PHASE_INITIAL,
            file: file || null,
            previewUrl: file ? window.URL.createObjectURL(file) : "",
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
                    updateState({
                        saveDetailsState: s.SAVE_DETAILS_SUCCEEDED,
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
                        initialMetadata: metadata,
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
