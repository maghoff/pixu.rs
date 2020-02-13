import DOM from './dom.js';

// Actions
const CROP_DRAG_START = "CROP_DRAG_START";
const CROP_DRAG_MOVE = "CROP_DRAG_MOVE";
const CROP_DRAG_STOP = "CROP_DRAG_STOP";
const CROP_DRAG_CANCEL = "CROP_DRAG_CANCEL";

function startHorizontalDrag(dx, imageRect, dragging, otherInitial) {
    return {
        type: CROP_DRAG_START,
        dx,
        imageRect,
        dragging,
        otherInitial,
    };
}

function moveHandle(clientX) {
    return {
        type: CROP_DRAG_MOVE,
        clientX
    };
}

function stopHorizontalDrag() {
    return {
        type: CROP_DRAG_STOP,
    }
}

function cancelHorizontalDrag() {
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
                    left: state.left,
                    right: state.right,
                },
                left: state.left,
                right: state.right,
            };

        case CROP_DRAG_MOVE:
            const targetPos = action.clientX + state.dx;
            let pos = (targetPos - state.imageRect.left) / state.imageRect.width;
            pos = Math.max(pos, 0);
            pos = Math.min(pos, 1);

            if (state.dragging == "left") {
                var left = pos;
                var right = Math.max(state.initial.right, left);
            } else {
                var right = pos;
                var left = Math.min(state.initial.left, right);
            }

            return {
                ...state,
                left,
                right,
            };

        case CROP_DRAG_STOP:
            return {
                left: state.left,
                right: state.right,
            };

        case CROP_DRAG_CANCEL:
            return state.initial;

        default:
            return state;
    }
}


function leftDxFromX(x) {
    // TODO Use .clientLeft and .clientWidth instead of .getBounding...?
    // TODO Consider using ev.target instead of DOM.crop
    const rect = DOM.crop.left.getBoundingClientRect();
    return rect.right - x;
}

function rightDxFromX(x) {
    const rect = DOM.crop.right.getBoundingClientRect();
    return rect.left - x;
}

export function init(dispatch) {
    const image = DOM.crop.horizontalImage;

    // Mouse interaction

    DOM.crop.leftHandle.addEventListener('mousedown', function (ev) {
        handleMouseDown(ev, leftDxFromX(ev.clientX), "left");
    });

    DOM.crop.rightHandle.addEventListener('mousedown', function (ev) {
        handleMouseDown(ev, rightDxFromX(ev.clientX), "right");
    });

    function handleMouseDown(ev, dx, dragging) {
        ev.preventDefault();
        ev.stopPropagation();

        window.addEventListener('mousemove', handleMove);
        window.addEventListener('mouseup', handleRelease);

        const imageRect = image.getBoundingClientRect();

        dispatch(startHorizontalDrag(dx, imageRect, dragging));
    }

    function handleMove(ev) {
        dispatch(moveHandle(ev.clientX));
    }

    function handleRelease(ev) {
        window.removeEventListener('mousemove', handleMove);
        window.removeEventListener('mouseup', handleRelease);

        dispatch(stopHorizontalDrag());
    }

    // Touch interaction

    DOM.crop.leftHandle.addEventListener('touchstart', function (ev) {
        handleTouchStart(ev, leftDxFromX(ev.touches[0].clientX), "left");
    });

    DOM.crop.rightHandle.addEventListener('touchstart', function (ev) {
        handleTouchStart(ev, rightDxFromX(ev.touches[0].clientX), "right");
    });

    function handleTouchStart(ev, dx, dragging) {
        ev.preventDefault();
        ev.stopPropagation();

        window.addEventListener('touchmove', handleTouchMove);
        window.addEventListener('touchend', handleTouchEnd);
        window.addEventListener('touchcancel', handleTouchCancel);

        const imageRect = image.getBoundingClientRect();

        dispatch(startHorizontalDrag(dx, imageRect, dragging));
    }

    function handleTouchMove(ev) {
        dispatch(moveHandle(ev.touches[0].clientX));
    }

    function handleTouchEnd(ev) {
        window.removeEventListener('touchmove', handleTouchMove);
        window.removeEventListener('touchend', handleTouchEnd);
        window.removeEventListener('touchcancel', handleTouchCancel);

        dispatch(stopHorizontalDrag());
    }

    function handleTouchCancel(ev) {
        window.removeEventListener('touchmove', handleTouchMove);
        window.removeEventListener('touchend', handleTouchEnd);
        window.removeEventListener('touchcancel', handleTouchCancel);

        dispatch(cancelHorizontalDrag());
    }
}
