import DOM from './dom.js';

// Actions
const CROP_DRAG_START = "CROP_DRAG_START";
const CROP_DRAG_MOVE = "CROP_DRAG_MOVE";
const CROP_DRAG_STOP = "CROP_DRAG_STOP";
const CROP_DRAG_CANCEL = "CROP_DRAG_CANCEL";

function startDrag(dx, imageRect, dragging) {
    return {
        type: CROP_DRAG_START,
        dx,
        imageRect,
        dragging,
    };
}

function moveHandle(clientX) {
    return {
        type: CROP_DRAG_MOVE,
        clientX
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

export function reducer(state, action) {
    switch (action.type) {
        case CROP_DRAG_START:
            return {
                dx: action.dx,
                imageRect: action.imageRect,
                dragging: action.dragging,
                initial: {
                    start: state.start,
                    end: state.end,
                },
                start: state.start,
                end: state.end,
            };

        case CROP_DRAG_MOVE:
            const targetPos = action.clientX + state.dx;
            let pos = (targetPos - state.imageRect.left) / state.imageRect.width;
            pos = Math.max(pos, 0);
            pos = Math.min(pos, 1);

            if (state.dragging == "start") {
                var start = pos;
                var end = Math.max(state.initial.end, start);
            } else {
                var end = pos;
                var start = Math.min(state.initial.start, end);
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
            };

        case CROP_DRAG_CANCEL:
            return state.initial;

        default:
            return state;
    }
}

export function init(dispatch) {
    const dom = DOM.crop.horizontal;

    function dxFromX(x, handle) {
        const edge = {
            "start": "right",
            "end": "left",
        };
        const rect = dom[handle].getBoundingClientRect();
        return rect[edge[handle]] - x;
    }

    // Mouse interaction
    dom.startHandle.addEventListener('mousedown', function (ev) {
        handleMouseDown(ev, "start");
    });

    dom.endHandle.addEventListener('mousedown', function (ev) {
        handleMouseDown(ev, "end");
    });

    function handleMouseDown(ev, handle) {
        ev.preventDefault();
        ev.stopPropagation();

        window.addEventListener('mousemove', handleMove);
        window.addEventListener('mouseup', handleRelease);

        // TODO Vertical support
        const dx = dxFromX(ev.clientX, handle);
        const imageRect = dom.image.getBoundingClientRect();

        dispatch(startDrag(dx, imageRect, handle));
    }

    function handleMove(ev) {
        dispatch(moveHandle(ev.clientX));
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

    dom.endHandle.addEventListener('touchstart', function (ev) {
        handleTouchStart(ev, "end");
    });

    function handleTouchStart(ev, handle) {
        ev.preventDefault();
        ev.stopPropagation();

        window.addEventListener('touchmove', handleTouchMove);
        window.addEventListener('touchend', handleTouchEnd);
        window.addEventListener('touchcancel', handleTouchCancel);

        // TODO Vertical support
        const dx = dxFromX(ev.touches[0].clientX, handle);
        const imageRect = dom.image.getBoundingClientRect();

        dispatch(startDrag(dx, imageRect, handle));
    }

    function handleTouchMove(ev) {
        dispatch(moveHandle(ev.touches[0].clientX));
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
