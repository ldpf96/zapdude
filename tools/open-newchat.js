(function () {
    // Clear the stuck search: escape twice, then open the New chat drawer.
    var active = document.activeElement;
    try {
        ['keydown', 'keyup'].forEach(function (type) {
            active && active.dispatchEvent(new KeyboardEvent(type, { key: 'Escape', bubbles: true }));
        });
    } catch (_escapeError) {}
    var button = document.querySelector('button[aria-label="New chat"]')
        || document.querySelector('button[aria-label*="new chat" i], button[aria-label*="chat nuevo" i]');
    if (!button) {
        return { ok: false, error: 'new_chat_button_not_found' };
    }
    button.click();
    return { ok: true, opened: true };
})()
