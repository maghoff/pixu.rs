import * as s from './states.js';

export default function (state) {
    return {
        ...state,
        formEnabled:
            (state.saveDetailsState != s.SAVE_DETAILS_IN_PROGRESS) &&
            (state.loadDetailsState == s.LOAD_DETAILS_READY)
    }
};
