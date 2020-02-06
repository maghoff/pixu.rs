const DOM = {
    preview: document.querySelector('.uploader-form--preview'),
    previewImages: document.querySelectorAll('.uploader-form--preview-image'),
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
    crop: {
        left: document.querySelector('.cropping--guide__left'),
        right: document.querySelector('.cropping--guide__right'),
        top: document.querySelector('.cropping--guide__top'),
        bottom: document.querySelector('.cropping--guide__bottom'),
    },
    details: {
        form: document.querySelector('.uploader-form--details'),
        detailsSubmission: document.querySelector('.uploader-form--details-submission'),
        submit: document.querySelector('.uploader-form--details button[type="submit"]'),
        status: document.querySelector('.uploader-form--status'),
    },
    email: {
        sendEmail: document.getElementById('send_email'),
        emailDetails: document.querySelector('.email-details'),
        title: document.getElementById('email-details--title'),
        messageInput: document.getElementById('email-details--message'),
        messagePreview: document.getElementById('message'),
        link: document.getElementById('link'),
    },
};

export default DOM;
