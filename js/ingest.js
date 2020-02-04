import DOM from './dom.js';
import render from './render.js';
import * as s from './states.js';

const initialState = {
    phase: s.PHASE_INITIAL,
    uploadPhase: s.UPLOAD_PHASE_INACTIVE,
    loadDetailsState: s.LOAD_DETAILS_READY,
    saveDetailsState: s.SAVE_DETAILS_INITIAL,
    previewUrl: "",
    sendEmail: DOM.email.sendEmail.defaultChecked,
    emailMessage: DOM.email.messageInput.defaultValue,
};
let state = initialState;


function setState(newState) {
    console.log(newState);
    render(state, newState);
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

const actions = {
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

                    // Abstraction leak, updating DOM outside of setState:
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
