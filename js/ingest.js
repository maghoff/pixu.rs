import dom from './dom.js';

const PHASE_INITIAL = 0;
const PHASE_PREVIEW = 1;
const PHASE_DETAILS = 2;

const UPLOAD_PHASE_INACTIVE = 0;
const UPLOAD_PHASE_IN_PROGRESS = 1;
const UPLOAD_PHASE_FINISHED = 2;

const UPLOAD_STATE_FAILURE = false;
const UPLOAD_STATE_SUCCESS = true;

const ERROR_TRY_AGAIN = "Prøv igjen 🤷";
const ERROR_CHECK_CONNECTIVITY = "🤔 Er du tilkoblet Internett?";

const LOAD_DETAILS_READY = 0;
const LOAD_DETAILS_PENDING = 1;
const LOAD_DETAILS_FAILED = 2;

const SAVE_DETAILS_INITIAL = 0;
const SAVE_DETAILS_IN_PROGRESS = 1;
const SAVE_DETAILS_SUCCEEDED = 2;
const SAVE_DETAILS_FAILED = 3;

const initialState = {
    phase: PHASE_INITIAL,
    uploadPhase: UPLOAD_PHASE_INACTIVE,
    loadDetailsState: LOAD_DETAILS_READY,
    saveDetailsState: SAVE_DETAILS_INITIAL,
    previewUrl: "",
};
let state = initialState;

function setState(newState) {
    console.log(newState);

    if (newState.phase != state.phase) {
        dom.phase.initial.style.display = (newState.phase == PHASE_INITIAL ? 'block' : 'none');
        dom.phase.preview.style.display = (newState.phase == PHASE_PREVIEW ? 'block' : 'none');
        dom.phase.details.style.display = (newState.phase == PHASE_DETAILS ? 'block' : 'none');

        const oldShowPreview = state.phase >= PHASE_PREVIEW;
        const newShowPreview = newState.phase >= PHASE_PREVIEW;
        if (newShowPreview != oldShowPreview) {
            dom.preview.style.display = newShowPreview ? "block" : "none";
        }
    }

    if (newState.previewUrl != state.previewUrl) {
        dom.preview.src = newState.previewUrl;
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

    let formEnabled =
        (state.saveDetailsState != SAVE_DETAILS_IN_PROGRESS) &&
        (state.loadDetailsState == LOAD_DETAILS_READY);
    let newFormEnabled =
        (newState.saveDetailsState != SAVE_DETAILS_IN_PROGRESS) &&
        (newState.loadDetailsState == LOAD_DETAILS_READY);

    if (newFormEnabled != formEnabled) {
        const disabledString = newFormEnabled ? "" : "disabled";
        for (let element of dom.details.form.elements) {
            element.disabled = disabledString;
        }
    }

    if (newState.saveDetailsState != state.saveDetailsState) {
        dom.details.submit.style.display =
            newState.saveDetailsState == SAVE_DETAILS_SUCCEEDED ? "none" : "block";

        if (newState.saveDetailsState == SAVE_DETAILS_SUCCEEDED) {
            dom.details.status.innerHTML =
                'Bildet er nå delt <a target=_blank href="' + newState.pixurUrl + '">her</a> 🙌';
        } else {
            let msg;
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
    const newState = { ...state, ...delta };
    setState(newState);
}

function gatherDetails() {
    const details = {
        metadata: {
            recipients: [],
        },
        send_email: document.getElementById("send_email").checked,
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

const actions = {
    selectFile: function (file) {
        updateState({
            phase: file ? PHASE_PREVIEW : PHASE_INITIAL,
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
                    hint: ERROR_CHECK_CONNECTIVITY,
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
            uploadError: null,
            pixurUrl: null,
            loadDetailsState: LOAD_DETAILS_READY,
        });
    },
    uploadFinished: function (location) {
        updateState({
            uploadPhase: UPLOAD_PHASE_FINISHED,
            uploadResult: UPLOAD_STATE_SUCCESS,
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
    selectExistingImage: function (pixurUrl, thumb) {
        fetch(pixurUrl + "/meta", {
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
                    return res.json();
                }
                catch (err) {
                    // Unexpected error
                    throw {
                        err: err,
                        hint: ERROR_TRY_AGAIN,
                    };
                }
            })
            .then(function (metadata) {
                try {
                    // Abstraction leak, updating DOM outside of setState:
                    setDetails(metadata);

                    updateState({
                        loadDetailsState: LOAD_DETAILS_READY,
                        initialMetadata: metadata,
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
                    loadDetailsState: LOAD_DETAILS_FAILED,
                    loadDetailsError: err,
                });
            });

        updateState({
            phase: PHASE_DETAILS,
            uploadPhase: UPLOAD_PHASE_FINISHED,
            uploadResult: UPLOAD_STATE_SUCCESS,
            pixurUrl,
            previewUrl: thumb,
            loadDetailsState: LOAD_DETAILS_PENDING,
            saveDetailsState: SAVE_DETAILS_INITIAL,
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

dom.details.form.addEventListener('submit', function (ev) {
    ev.preventDefault();
    ev.stopPropagation();
    actions.submitDetails();
});

document.querySelector('.thumbnails').addEventListener('click', function (ev) {
    ev.preventDefault();
    ev.stopPropagation();

    let li = ev.target;
    while (li && li.tagName != "LI") {
        li = li.parentNode;
    }

    if (!li) return;

    let pixurUrl = li.querySelector("a").href;
    let thumb = li.querySelector("img").src;

    actions.selectExistingImage(pixurUrl, thumb);
});

actions.selectFile(dom.fileInput.files[0]);
