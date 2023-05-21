export function getSelectedText() {
    const doc = window.getSelection().getRangeAt(0).cloneContents(),
        nodes = doc.querySelectorAll('tr');
    let text = '';

    if (nodes.length === 0) {
        text = doc.textContent;
    } else {
        [].forEach.call(nodes, function(tr, i) {
            // Get last column's text (since that has the text we want to copy).
            const td = tr.cells[tr.cells.length - 1];
            text += (i ? '\n' : '') + td.textContent;
        });
    }

    return text;
}
