let pendingResize = false;
let pendingUpdate = false;
let anchorElement = null;
let anchorElementRatio = null;

function updateInView() {
    if (pendingUpdate) return;
    pendingUpdate = true;

    window.requestAnimationFrame(function () {
        if (pendingResize) {
            const el = anchorElement;
            const top = el.offsetTop;
            const height = el.clientHeight;

            const anchorElementPos = top + height * anchorElementRatio;
            window.scrollTo(window.scrollX, anchorElementPos - window.innerHeight / 2);

            pendingResize = false;
        }

        pendingUpdate = false;

        const viewTop = window.scrollY;
        const viewBottom = viewTop + window.innerHeight;

        const anchorY = (viewTop + viewBottom) / 2.;

        const elements = document.querySelectorAll(".photo");
        for (el of elements) {
            const top = el.offsetTop;
            const bottom = top + el.clientHeight;

            if ((top <= anchorY) && (anchorY < bottom)) {
                anchorElement = el;
                anchorElementRatio = (anchorY - top) / (bottom - top);
            }

            const inView = (bottom >= viewTop) && (top <= viewBottom);
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
