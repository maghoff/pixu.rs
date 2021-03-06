import * as crop from './crop.js';
import * as s from './states.js';
import DOM from './dom.js';
import render from './render.js';
import viewModel from './viewmodel.js';

export const initialState = {
    phase: s.PHASE_INITIAL,
    uploadPhase: s.UPLOAD_PHASE_INACTIVE,
    loadDetailsState: s.LOAD_DETAILS_READY,
    saveDetailsState: s.SAVE_DETAILS_INITIAL,
    previewUrl: "",
    savedComment: "",
    comment: "",
    sendEmail: DOM.email.sendEmail.defaultChecked,
    emailMessage: DOM.email.messageInput.defaultValue,
    savedRecipients: [],
    recipients: [],
    cropHorizontal: {},
    cropVertical: {},
};
export let state = initialState;

let lastViewModel = viewModel(state);

export function setState(newState) {
    let newViewModel = viewModel(newState);
    console.log(newViewModel);
    render(lastViewModel, newViewModel);

    state = newState;
    lastViewModel = newViewModel;
}

export function updateState(delta) {
    const newState = { ...state, ...delta };
    setState(newState);
}

function rootReducer(state, action) {
    return {
        ...state,
        cropHorizontal: crop.reducer(state.cropHorizontal, action),
        cropVertical: crop.reducer(state.cropVertical, action),
    };
}

export function dispatch(action) {
    let newState = rootReducer(state, action);
    setState(newState);
}
