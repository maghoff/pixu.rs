import * as crop from './crop.js';
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
    cropHorizontal: {},
    cropVertical: {},
};
export let state = initialState;

export function setState(newState) {
    console.log(newState);
    render(state, newState);
    state = newState;
}

export function updateState(delta) {
    const newState = { ...state, ...delta };
    setState(newState);
}

function rootReducer(state, action) {
    return {
        ...state,
        cropHorizontal: crop.reducer(state.cropHorizontal, action),
    };
}

export function dispatch(action) {
    console.log(action);
    let newState = rootReducer(state, action);
    setState(newState);
}
