function lineNumbersRange() {
    // Get the line number(s) from the hash
    if (location.hash.startsWith("#")) {
        const range = location.hash.substring(1).split("-");
        const start = parseInt(range[0]);
        const end = range.length > 1 ? parseInt(range[1]) : start;

        if (!isNaN(start) && !isNaN(end)) {
            return { start, end, isValidId: range.length === 1 };
        }
    }

    return null;
}

function updateLineNumbers() {
    // Remove the highlighted class from all line numbers
    const elements = document.getElementsByClassName("line-number-highlighted");
    while (elements.length > 0) {
        elements[0].classList.remove("line-number-highlighted");
    }

    // Highlight the line numbers based on the hash
    const range = lineNumbersRange();
    if (range) {
        for (let n = range.start; n <= range.end; n++) {
            const element = document.getElementById(`${n}`);
            if (element) {
                element.classList.add("line-number-highlighted");
            }
        }
    }
}

function jumpToLineNumbersHash() {
    const range = lineNumbersRange();
    if (!range) {
        return;
    }

    // If the hash is a valid id, do nothing here. The browser will jump to the id.
    if (range.isValidId) {
        return;
    }

    // Jump to the first line number
    const element = document.getElementById(`${range.start}`);
    if (element) {
        element.scrollIntoView();
    }
}

window.addEventListener(
    "hashchange",
    updateLineNumbers,
);

window.addEventListener('click', function (e) {
    // Check if target is a line-number
    if (!e.target.classList.contains('line-number')) {
        return;
    }

    // Prevent default behavior (jumping to the anchor)
    e.preventDefault();

    // Calculate the new hash
    let newHash = e.target.hash;
    if (e.shiftKey) {
        const currentRange = lineNumbersRange();
        if (currentRange) {
            const clicked = parseInt(newHash.substring(1));
            if (clicked < currentRange.start) {
                newHash = `#${clicked}-${currentRange.end}`;
            } else {
                newHash = `#${currentRange.start}-${clicked}`;
            }
        }
    }

    // Replace url
    const url = new URL(window.location);
    url.hash = newHash;
    history.replaceState(null, '', url);

    // Update the line numbers
    updateLineNumbers();
});

// On page load, update the line numbers
updateLineNumbers();

// On page load, jump to hash
jumpToLineNumbersHash();
