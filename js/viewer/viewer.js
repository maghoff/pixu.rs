let pendingResize = false;
let pendingUpdate = false;
let anchorElement = null;
let anchorElementRatio = null;

const viewportSizeElement = document.querySelector(".viewport-size");
let vh = viewportSizeElement.clientHeight, vw = viewportSizeElement.clientWidth;

function handlePendingResize() {
    const newVh = viewportSizeElement.clientHeight, newVw = viewportSizeElement.clientWidth;
    if (vh == newVh && vw == newVw) return;
    vh = newVh;
    vw = newVw;

    const el = anchorElement;
    if (el) {
        const top = el.offsetTop;
        const height = el.clientHeight;

        const anchorElementPos = top + height * anchorElementRatio;
        window.scrollTo(window.scrollX, anchorElementPos - vh / 2);
    }
}

function updateInView() {
    if (pendingUpdate) return;
    pendingUpdate = true;

    window.requestAnimationFrame(function () {
        if (pendingResize) {
            pendingResize = false;
            handlePendingResize();
        }

        pendingUpdate = false;

        // Add some padding to preload photos that are almost in view
        const paddingTop = 100;
        const paddingBottom = 300;

        const viewTop = window.scrollY;
        const viewBottom = viewTop + vh;

        const anchorY = (viewTop + viewBottom) / 2.;

        const elements = document.querySelectorAll(".photo");
        for (el of elements) {
            const top = el.offsetTop;
            const bottom = top + el.clientHeight;

            if ((top <= anchorY) && (anchorY < bottom)) {
                anchorElement = el;
                anchorElementRatio = (anchorY - top) / (bottom - top);
            }

            const inView = (bottom >= viewTop - paddingTop) && (top <= viewBottom + paddingBottom);
            if (inView) el.classList.add("in-view");
            else el.classList.remove("in-view");
        }
    });
}

function handleResize() {
    pendingResize = true;
    updateInView();
}

window.addEventListener('scroll', updateInView);
window.addEventListener('resize', handleResize);

updateInView();
