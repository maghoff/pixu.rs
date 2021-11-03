async function saveSeries(series, recipients) {
    let res =
        await fetch("", {
            method: 'POST',
            body: JSON.stringify({ series, recipients }),
            headers: {
                'Content-Type': 'application/json'
            },
            credentials: 'same-origin',
            redirect: 'follow',
        })
            .catch(err => { throw "Noe gikk galt. Er du tilkoblet Internett?" });

    if (!res.ok) {
        throw `Noe gikk galt. Feilkode ${res.status} ${res.statusText}`;
    }

    alert("Lagret ✔");
}

document.getElementById("form").addEventListener('submit', function (ev) {
    ev.preventDefault();
    ev.stopPropagation();

    const series = [];
    for (let item of document.querySelectorAll(".series--item")) {
        const i = {
            pixurs_id: item.querySelector('[name="pixurs_id"]').value,
            comment: item.querySelector('[name="comment"]').value || null,
            comment_position: item.querySelector('[type="radio"]:checked').value,
        };
        series.push(i);
    }

    const recipients = [];
    const rec = document.querySelector(".uploader-form--recipients").selectedOptions;
    for (let i = 0; i < rec.length; ++i) {
        recipients.push(rec[i].value);
    }

    saveSeries(series, recipients)
        .catch(err => alert(err));
});

document.getElementById("form").addEventListener('click', function (ev) {
    if (ev.target.classList.contains("series--delete")) {
        ev.preventDefault();
        ev.stopPropagation();

        const item = ev.target.parentNode;
        const list = item.parentNode;

        list.removeChild(item);
    }
});

document.getElementById("add-photo").addEventListener('click', function (ev) {
    ev.preventDefault();
    ev.stopPropagation();

    const list = document.querySelector(".series");
    const template = document.querySelector("template");

    const pixursId = prompt("Pixur ID");

    const item = template.content.cloneNode(true);
    item.querySelector('input[name="pixurs_id"]').setAttribute('value', pixursId);
    for (let radio of item.querySelectorAll('input[type="radio"]')) {
        radio.id = radio.id.replace("PIXURS_ID", pixursId);
        radio.name = radio.name.replace("PIXURS_ID", pixursId);
    }
    for (let label of item.querySelectorAll('label')) {
        let n = label.getAttribute('for').replace("PIXURS_ID", pixursId);
        label.setAttribute('for', n);
    }

    list.appendChild(item);
});



// Drag sorting due to https://stackoverflow.com/a/28962290
let listRoot = document.querySelector("ul");
let draggedItem;

function dragOver(e) {
    let over = e.target;
    while (over && (over.parentNode !== listRoot)) {
        over = over.parentNode;
    }
    if (!over) return;

    if (isBefore(draggedItem, over)) {
        listRoot.insertBefore(draggedItem, over);
    } else {
        listRoot.insertBefore(draggedItem, over.nextSibling);
    }
}

function dragStart(e) {
    e.dataTransfer.effectAllowed = "move";
    draggedItem = e.target;
    setTimeout(() => draggedItem.classList.add("dragging"), 0);
}

function dragEnd(ev) {
    if (draggedItem) {
        draggedItem.classList.remove("dragging");
        draggedItem = null;
    }
}

function isBefore(el1, el2) {
    if (el2.parentNode === el1.parentNode) {
        for (var cur = el1.previousSibling; cur && cur.nodeType !== 9; cur = cur.previousSibling) {
            if (cur === el2) {
                return true;
            }
        }
    }
    return false;
}

document.addEventListener('dragstart', dragStart);
document.addEventListener('dragover', dragOver);
document.addEventListener('dragend', dragEnd);
