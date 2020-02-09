import DOM from './dom.js';
import { actions, updateState, state } from './actions.js';

function handleLeftMousedown(ev) {
    if (state.cropLeftDrag) return;

    ev.preventDefault();
    ev.stopPropagation();

    const rect = DOM.crop.leftHandle.getBoundingClientRect();
    const dx = rect.right - ev.clientX;

    updateState({
        cropLeftDrag: {
            dx,
            startCropRight: state.cropRight,
            imageRect: DOM.crop.horizontalImage.getBoundingClientRect(),
        }
    });
}

DOM.crop.leftHandle.addEventListener('touchstart', handleLeftMousedown);
DOM.crop.leftHandle.addEventListener('mousedown', handleLeftMousedown);

function handleRightMousedown(ev) {
    if (state.cropRightDrag) return;

    ev.preventDefault();
    ev.stopPropagation();

    const rect = DOM.crop.rightHandle.getBoundingClientRect();
    const dx = rect.left - ev.clientX;

    updateState({
        cropRightDrag: {
            dx,
            startCropLeft: state.cropLeft,
            imageRect: DOM.crop.horizontalImage.getBoundingClientRect(),
        }
    });
}
DOM.crop.rightHandle.addEventListener('touchstart', handleRightMousedown);
DOM.crop.rightHandle.addEventListener('mousedown', handleRightMousedown);

window.addEventListener('mousemove', function (ev) {
    if (state.cropLeftDrag) {
        const targetPos = ev.clientX + state.cropLeftDrag.dx;

        let left = (targetPos - state.cropLeftDrag.imageRect.left) / state.cropLeftDrag.imageRect.width;
        left = Math.max(left, 0);
        left = Math.min(left, 1);
        let right = Math.max(state.cropLeftDrag.startCropRight, left);

        updateState({
            cropLeft: left,
            cropRight: right,
        });
    }

    if (state.cropRightDrag) {
        const targetPos = ev.clientX + state.cropRightDrag.dx;

        let right = (targetPos - state.cropRightDrag.imageRect.left) / state.cropRightDrag.imageRect.width;
        right = Math.max(right, 0);
        right = Math.min(right, 1);
        let left = Math.min(state.cropRightDrag.startCropLeft, right);

        updateState({
            cropLeft: left,
            cropRight: right,
        });
    }
});

window.addEventListener('mouseup', function (ev) {
    updateState({
        cropLeftDrag: null,
        cropRightDrag: null,
    });
});
