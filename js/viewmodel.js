import * as s from './states.js';

export default function (state) {
    return {
        ...state,
        showPreview: state.phase >= s.PHASE_PREVIEW,
        formEnabled:
            (state.saveDetailsState != s.SAVE_DETAILS_IN_PROGRESS) &&
            (state.loadDetailsState == s.LOAD_DETAILS_READY)
    }
};
