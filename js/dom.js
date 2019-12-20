const dom = {
    preview: document.querySelector('.uploader-form--preview'),
    uploaderForm: document.getElementById('uploader-form'),
    fileInput: document.querySelector('#uploader-form input[type="file"]'),
    phase: {
        initial: document.querySelector('.uploader-form--phase-initial'),
        preview: document.querySelector('.uploader-form--phase-preview'),
        details: document.querySelector('.uploader-form--phase-details'),
    },
    uploader: {
        errorMessage: document.querySelector('.uploader-form--error-message'),
        uploadError: document.querySelector('.uploader-form--upload-error'),
        statusUploading: document.querySelector('.uploader-form--status__uploading'),
        statusUploaded: document.querySelector('.uploader-form--status__uploaded'),
        pixurUrl: document.querySelector('.uploader-form--url'),
    },
    details: {
        form: document.querySelector('.uploader-form--details'),
        detailsSubmission: document.querySelector('.uploader-form--details-submission'),
        submit: document.querySelector('.uploader-form--details button[type="submit"]'),
        status: document.querySelector('.uploader-form--status'),
    },
};


export default dom;
