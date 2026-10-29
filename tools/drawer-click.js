(function () {
    var drawer = document.querySelector('div[data-testid="new-chat-drawer"]');
    if (!drawer) {
        return { ok: false, error: 'no_drawer' };
    }
    var rows = drawer.querySelectorAll('div[data-testid="cell-frame-container"]');
    var clicked = null;
    // Prefer a row that spells a match for the typed number.
    for (var index = 0; index < rows.length; index++) {
        if (/[Mm]atches this number|coincide con este n/.test(String(rows[index].textContent || ''))) {
            clicked = rows[index];
            break;
        }
    }
    if (!clicked && rows.length) {
        clicked = rows[0];
    }
    if (!clicked) {
        return { ok: false, error: 'no_result_row' };
    }
    var target = clicked;
    while (target && target !== drawer && target.getAttribute('role') !== 'row') {
        target = target.parentElement;
        if (target && target.tagName === 'BODY') {
            break;
        }
    }
    (target && target !== drawer ? target : clicked).click();
    return { ok: true, clicked: true };
})()
