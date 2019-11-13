// The uploader will work with JS only. Too much mucking about to parse
// multipart messages.

var preview = document.querySelector('.uploader-form--preview');
var form = document.getElementById('uploader-form');
var fileInput = form.querySelector('input[type="file"]');

var PHASE_INITIAL = 0;
var PHASE_PREVIEW = 1;
var PHASE_DETAILS = 2;

var UPLOAD_PHASE_INACTIVE = 0;
var UPLOAD_PHASE_IN_PROGRESS = 1;
var UPLOAD_PHASE_FINISHED = 2;

var UPLOAD_STATE_FAILURE = false;
var UPLOAD_STATE_SUCCESS = true;

var UPLOAD_ERROR_TRY_AGAIN = "Prøv igjen 🤷";
var UPLOAD_ERROR_CHECK_CONNECTIVITY = "🤔 Er du tilkoblet Internett?";

var initialState = {
    phase: PHASE_INITIAL,
    uploadPhase: UPLOAD_PHASE_INACTIVE,
};
var state = initialState;

function setState(newState) {
    console.log(newState);

    if (newState.phase != state.phase) {
        document.querySelector('.uploader-form--phase-initial').style.display =
            (newState.phase == PHASE_INITIAL ? 'block' : 'none');
        document.querySelector('.uploader-form--phase-preview').style.display =
            (newState.phase == PHASE_PREVIEW ? 'block' : 'none');
        document.querySelector('.uploader-form--phase-details').style.display =
            (newState.phase == PHASE_DETAILS ? 'block' : 'none');

        var oldShowPreview = state.phase >= PHASE_PREVIEW;
        var newShowPreview = newState.phase >= PHASE_PREVIEW;
        if (newShowPreview != oldShowPreview) {
            preview.style.display = newShowPreview ? "block" : "none";
        }
    }

    if (newState.file != state.file) {
        preview.src = newState.file ? window.URL.createObjectURL(newState.file) : "";
    }

    if (newState.uploadError != state.uploadError) {
        if (newState.uploadError) {
            document.querySelector('.uploader-form--error-message').textContent =
                newState.uploadError.hint;
        }

        document.querySelector('.uploader-form--upload-error').style.display =
            newState.uploadError ? 'block' : 'none';
    }

    if (newState.uploadResult !== state.uploadResult) {
        document.querySelector('.uploader-form--status').textContent =
            newState.uploadResult == UPLOAD_STATE_SUCCESS ?
                "Er alt klart da?" : "Nå trenger vi bare å vente på bildeopplastingen…";

        document.querySelector('.uploader-form--details button[type="submit"]').disabled =
            newState.uploadResult != UPLOAD_STATE_SUCCESS;
    }

    state = newState;
}

function updateState(delta) {
    var newState = { ...state, ...delta };
    setState(newState);
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
                    hint: UPLOAD_ERROR_CHECK_CONNECTIVITY,
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
                        hint: UPLOAD_ERROR_TRY_AGAIN,
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
    }
};

fileInput.addEventListener('change', function (ev) {
    ev.preventDefault();
    ev.stopPropagation();
    actions.selectFile(fileInput.files[0]);
});

form.addEventListener('reset', function (ev) {
    actions.reset();
});

form.addEventListener('submit', function (ev) {
    ev.preventDefault();
    ev.stopPropagation();
    actions.upload(state.file);
});

actions.selectFile(fileInput.files[0]);
