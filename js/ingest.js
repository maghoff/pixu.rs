// The uploader will work with JS only. Too much mucking about to parse
// multipart messages.

import dom from './dom.js';

var PHASE_INITIAL = 0;
var PHASE_PREVIEW = 1;
var PHASE_DETAILS = 2;

var UPLOAD_PHASE_INACTIVE = 0;
var UPLOAD_PHASE_IN_PROGRESS = 1;
var UPLOAD_PHASE_FINISHED = 2;

var UPLOAD_STATE_FAILURE = false;
var UPLOAD_STATE_SUCCESS = true;

var ERROR_TRY_AGAIN = "Prøv igjen 🤷";
var ERROR_CHECK_CONNECTIVITY = "🤔 Er du tilkoblet Internett?";

var SAVE_DETAILS_INITIAL = 0;
var SAVE_DETAILS_IN_PROGRESS = 1;
var SAVE_DETAILS_SUCCEEDED = 2;
var SAVE_DETAILS_FAILED = 3;

var initialState = {
    phase: PHASE_INITIAL,
    uploadPhase: UPLOAD_PHASE_INACTIVE,
    saveDetailsState: SAVE_DETAILS_INITIAL,
};
var state = initialState;

function setState(newState) {
    console.log(newState);

    if (newState.phase != state.phase) {
        dom.phase.initial.style.display = (newState.phase == PHASE_INITIAL ? 'block' : 'none');
        dom.phase.preview.style.display = (newState.phase == PHASE_PREVIEW ? 'block' : 'none');
        dom.phase.details.style.display = (newState.phase == PHASE_DETAILS ? 'block' : 'none');

        var oldShowPreview = state.phase >= PHASE_PREVIEW;
        var newShowPreview = newState.phase >= PHASE_PREVIEW;
        if (newShowPreview != oldShowPreview) {
            dom.preview.style.display = newShowPreview ? "block" : "none";
        }
    }

    if (newState.file != state.file) {
        dom.preview.src = newState.file ? window.URL.createObjectURL(newState.file) : "";
    }

    if (newState.uploadError != state.uploadError) {
        if (newState.uploadError) {
            dom.uploader.errorMessage.textContent = newState.uploadError.hint;
        }

        dom.uploader.uploadError.style.display = newState.uploadError ? 'block' : 'none';
    }

    if (newState.uploadResult !== state.uploadResult) {
        dom.details.detailsSubmission.style.display =
            newState.uploadResult == UPLOAD_STATE_SUCCESS ? "block" : "none";
    }

    if (newState.saveDetailsState != state.saveDetailsState) {
        let formEnabled = newState.saveDetailsState != SAVE_DETAILS_IN_PROGRESS;

        dom.details.submit.disabled = formEnabled ? "" : "disabled";

        dom.details.submit.style.display =
            newState.saveDetailsState == SAVE_DETAILS_SUCCEEDED ? "none" : "block";

        if (newState.saveDetailsState == SAVE_DETAILS_SUCCEEDED) {
            dom.details.status.innerHTML =
                'Bildet er nå delt <a href="' + newState.uploadLocation + '">her</a> 🙌';
        } else {
            var msg;
            switch (newState.saveDetailsState) {
                case SAVE_DETAILS_INITIAL: msg = "Er alt klart da?"; break;
                case SAVE_DETAILS_IN_PROGRESS: msg = "Deler…"; break;
                case SAVE_DETAILS_FAILED: msg = "😕 Noe skar seg. " + newState.saveDetailsError.hint; break;
            }

            dom.details.status.textContent = msg;
        }
    }

    state = newState;
}

function updateState(delta) {
    var newState = { ...state, ...delta };
    setState(newState);
}

function gatherDetails() {
    var details = {
        metadata: {
            recipients: [],
        },
        send_email: document.getElementById("send_email").checked,
    };

    var s = document.querySelector(".uploader-form--recipients").selectedOptions;
    for (var i = 0; i < s.length; ++i) {
        details.metadata.recipients.push(s[i].value);
    }

    return details;
}

var actions = {
    selectFile: function (file) {
        updateState({
            phase: file ? PHASE_PREVIEW : PHASE_INITIAL,
            file: file || null,
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
                    hint: ERROR_CHECK_CONNECTIVITY,
                };
            })
            .then(function (res) {
                try {
                    if (!res.ok) {
                        throw "Unexpected status code: " + res.status + " " + res.statusText;
                    }

                    var location = res.headers.get('location');
                    if (!location) {
                        throw "Missing Location header in server response";
                    }

                    actions.uploadFinished(location);
                }
                catch (err) {
                    // Unexpected error
                    throw {
                        err: err,
                        hint: ERROR_TRY_AGAIN,
                    };
                }
            })
            .catch(function (err) {
                updateState({
                    uploadPhase: UPLOAD_PHASE_FINISHED,
                    uploadResult: UPLOAD_STATE_FAILURE,
                    uploadError: err,
                });
            });

        updateState({
            phase: PHASE_DETAILS,
            uploadPhase: UPLOAD_PHASE_IN_PROGRESS,
            uploadResult: null,
            uploadLocation: null,
            uploadError: null,
        });
    },
    uploadFinished: function (location) {
        updateState({
            uploadPhase: UPLOAD_PHASE_FINISHED,
            uploadResult: UPLOAD_STATE_SUCCESS,
            uploadLocation: location,
        });
    },
    submitDetails: function () {
        let details = gatherDetails();

        fetch(state.uploadLocation + "/meta", {
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
                    hint: ERROR_CHECK_CONNECTIVITY,
                };
            })
            .then(function (res) {
                try {
                    if (!res.ok) {
                        throw "Unexpected status code: " + res.status + " " + res.statusText;
                    }
                    updateState({
                        saveDetailsState: SAVE_DETAILS_SUCCEEDED,
                    });
                }
                catch (err) {
                    // Unexpected error
                    throw {
                        err: err,
                        hint: ERROR_TRY_AGAIN,
                    };
                }
            })
            .catch(function (err) {
                updateState({
                    saveDetailsState: SAVE_DETAILS_FAILED,
                    saveDetailsError: err,
                });
            });

        updateState({
            saveDetailsState: SAVE_DETAILS_IN_PROGRESS,
        });
    },
};

dom.fileInput.addEventListener('change', function (ev) {
    ev.preventDefault();
    ev.stopPropagation();
    actions.selectFile(dom.fileInput.files[0]);
});

dom.uploaderForm.addEventListener('reset', function (ev) {
    actions.reset();
});

dom.uploaderForm.addEventListener('submit', function (ev) {
    ev.preventDefault();
    ev.stopPropagation();
    actions.upload(state.file);
});

document.getElementById("uploader-form--add-recipient").addEventListener('click', function (ev) {
    var email = prompt("Epostadresse");
    if (email) {
        var opt = document.createElement("option");
        opt.setAttribute("value", email);
        opt.textContent = email;
        opt.setAttribute("selected", "selected");

        var sel = document.querySelector(".uploader-form--recipients");
        sel.appendChild(opt);
        sel.size = sel.options.length;
    }
});

document.getElementById('uploader-form--details').addEventListener('submit', function (ev) {
    ev.preventDefault();
    ev.stopPropagation();
    actions.submitDetails();
});

actions.selectFile(fileInput.files[0]);
