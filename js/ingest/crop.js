import DOM from './dom.js';
import { LOAD_DETAILS_READY } from './states.js';

// Actions
const CROP_DRAG_START = "CROP_DRAG_START";
const CROP_DRAG_MOVE = "CROP_DRAG_MOVE";
const CROP_DRAG_STOP = "CROP_DRAG_STOP";
const CROP_DRAG_CANCEL = "CROP_DRAG_CANCEL";

function startDrag(delta, imageStart, imageSize, dragging) {
    return {
        type: CROP_DRAG_START,
        delta,
        imageStart,
        imageSize,
        dragging,
    };
}

function moveHandle(clientPos) {
    return {
        type: CROP_DRAG_MOVE,
        clientPos
    };
}

function stopDrag() {
    return {
        type: CROP_DRAG_STOP,
    }
}

function cancelDrag() {
    return {
        type: CROP_DRAG_CANCEL,
    }
}

// ---

export function render(prev, next) {
    function renderCrop(dom, prev, next, startAnchor, endAnchor) {
        if (next.start !== prev.start) {
            dom.start.style[startAnchor] = ((1 - next.start) * 100) + "%";
        }
        if (next.end !== prev.end) {
            dom.end.style[endAnchor] = (next.end * 100) + "%";
        }
        if ((next.start !== prev.start) || (next.end !== prev.end)) {
            dom.middle.style[endAnchor] = ((next.start + next.end) / 2 * 100) + "%";
        }

        const nextStartDrag = next.dragging == "start";
        const prevStartDrag = prev.dragging == "start";
        if (nextStartDrag !== prevStartDrag) {
            const action = nextStartDrag ? "add" : "remove";
            dom.startHandle.classList[action]("cropping--handle__active");
        }

        const nextMiddleDrag = next.dragging == "middle";
        const prevMiddleDrag = prev.dragging == "middle";
        if (nextMiddleDrag !== prevMiddleDrag) {
            const action = nextMiddleDrag ? "add" : "remove";
            dom.middleHandle.classList[action]("cropping--handle__active");
        }

        const nextEndDrag = next.dragging == "end";
        const prevEndDrag = prev.dragging == "end";
        if (nextEndDrag !== prevEndDrag) {
            const action = nextEndDrag ? "add" : "remove";
            dom.endHandle.classList[action]("cropping--handle__active");
        }
    }

    if (next.formEnabled != prev.formEnabled) {
        const action = next.formEnabled ? "removeAttribute" : "setAttribute";
        const elements = [
            DOM.crop.horizontal.start,
            DOM.crop.horizontal.startHandle,
            DOM.crop.horizontal.middle,
            DOM.crop.horizontal.middleHandle,
            DOM.crop.horizontal.end,
            DOM.crop.horizontal.endHandle,
            DOM.crop.vertical.start,
            DOM.crop.vertical.startHandle,
            DOM.crop.vertical.middle,
            DOM.crop.vertical.middleHandle,
            DOM.crop.vertical.end,
            DOM.crop.vertical.endHandle,
        ];
        for (let element of elements) {
            element[action]("disabled", "disabled");
        }
    }

    const prevInitialized = prev.loadDetailsState == LOAD_DETAILS_READY;
    const nextInitialized = next.loadDetailsState == LOAD_DETAILS_READY;

    if (nextInitialized != prevInitialized) {
        const action = nextInitialized ? "remove" : "add";
        DOM.crop.horizontal.root.classList[action]("cropping__uninitialized");
        DOM.crop.vertical.root.classList[action]("cropping__uninitialized");
    }

    renderCrop(DOM.crop.horizontal, prev.cropHorizontal, next.cropHorizontal, "right", "left");
    renderCrop(DOM.crop.vertical, prev.cropVertical, next.cropVertical, "bottom", "top");
}

export function reducer(state, action) {
    switch (action.type) {
        case CROP_DRAG_START:
            return {
                delta: action.delta,
                imageStart: action.imageStart,
                imageSize: action.imageSize,
                dragging: action.dragging,
                initial: {
                    start: state.start,
                    end: state.end,
                },
                start: state.start,
                end: state.end,
                savedStart: state.savedStart,
                savedEnd: state.savedEnd,
            };

        case CROP_DRAG_MOVE:
            const targetPos = action.clientPos + state.delta;
            let pos = (targetPos - state.imageStart) / state.imageSize;
            pos = Math.max(pos, 0);
            pos = Math.min(pos, 1);

            if (state.dragging == "start") {
                var start = pos;
                var end = Math.max(state.initial.end, start);
            } else if (state.dragging == "end") {
                var end = pos;
                var start = Math.min(state.initial.start, end);
            } else if (state.dragging == "middle") {
                let hwidth = (state.initial.end - state.initial.start) / 2;
                hwidth = Math.min(pos, hwidth);
                hwidth = Math.min(1 - pos, hwidth);
                var start = pos - hwidth;
                var end = pos + hwidth;
            } else {
                console.error(`Invalid dragging handle: ${state.dragging}`);
                return state;
            }

            return {
                ...state,
                start,
                end,
            };

        case CROP_DRAG_STOP:
            return {
                start: state.start,
                end: state.end,
                savedStart: state.savedStart,
                savedEnd: state.savedEnd,
            };

        case CROP_DRAG_CANCEL:
            return state.initial;

        default:
            return state;
    }
}

const EDGES_BY_AXIS = {
    "horizontal": {
        "start": "right",
        "middle": "left",
        "end": "left",
    },
    "vertical": {
        "start": "bottom",
        "middle": "top",
        "end": "top",
    },
};

const CLIENTPOS_BY_AXIS = {
    "horizontal": "clientX",
    "vertical": "clientY",
};

const EXTENTS_BY_AXIS = {
    "horizontal": ["left", "width"],
    "vertical": ["top", "height"],
};

export function init(dispatch, dom, axis) {
    if (!EDGES_BY_AXIS.hasOwnProperty(axis)) {
        throw new Error("Invalid axis");
    }

    const edge = EDGES_BY_AXIS[axis];
    const clientPos = CLIENTPOS_BY_AXIS[axis];
    const extents = EXTENTS_BY_AXIS[axis];

    function deltaFromPos(pos, handle) {
        const rect = dom[handle].getBoundingClientRect();
        return rect[edge[handle]] - pos;
    }

    // Mouse interaction
    dom.startHandle.addEventListener('mousedown', function (ev) {
        handleMouseDown(ev, "start");
    });

    dom.middleHandle.addEventListener('mousedown', function (ev) {
        handleMouseDown(ev, "middle");
    });

    dom.endHandle.addEventListener('mousedown', function (ev) {
        handleMouseDown(ev, "end");
    });

    function handleMouseDown(ev, handle) {
        if (ev.target.getAttribute("disabled")) {
            return;
        }

        ev.preventDefault();
        ev.stopPropagation();

        window.addEventListener('mousemove', handleMove);
        window.addEventListener('mouseup', handleRelease);

        const delta = deltaFromPos(ev[clientPos], handle);
        const imageRect = dom.image.getBoundingClientRect();

        dispatch(startDrag(delta, imageRect[extents[0]], imageRect[extents[1]], handle));
    }

    function handleMove(ev) {
        dispatch(moveHandle(ev[clientPos]));
    }

    function handleRelease(ev) {
        window.removeEventListener('mousemove', handleMove);
        window.removeEventListener('mouseup', handleRelease);

        dispatch(stopDrag());
    }

    // Touch interaction

    dom.startHandle.addEventListener('touchstart', function (ev) {
        handleTouchStart(ev, "start");
    });

    dom.middleHandle.addEventListener('touchstart', function (ev) {
        handleTouchStart(ev, "middle");
    });

    dom.endHandle.addEventListener('touchstart', function (ev) {
        handleTouchStart(ev, "end");
    });

    function handleTouchStart(ev, handle) {
        if (ev.target.getAttribute("disabled")) {
            return;
        }

        ev.preventDefault();
        ev.stopPropagation();

        window.addEventListener('touchmove', handleTouchMove);
        window.addEventListener('touchend', handleTouchEnd);
        window.addEventListener('touchcancel', handleTouchCancel);

        const delta = deltaFromPos(ev.touches[0][clientPos], handle);
        const imageRect = dom.image.getBoundingClientRect();

        dispatch(startDrag(delta, imageRect[extents[0]], imageRect[extents[1]], handle));
    }

    function handleTouchMove(ev) {
        dispatch(moveHandle(ev.touches[0][clientPos]));
    }

    function handleTouchEnd(ev) {
        window.removeEventListener('touchmove', handleTouchMove);
        window.removeEventListener('touchend', handleTouchEnd);
        window.removeEventListener('touchcancel', handleTouchCancel);

        dispatch(stopDrag());
    }

    function handleTouchCancel(ev) {
        window.removeEventListener('touchmove', handleTouchMove);
        window.removeEventListener('touchend', handleTouchEnd);
        window.removeEventListener('touchcancel', handleTouchCancel);

        dispatch(cancelDrag());
    }
}
