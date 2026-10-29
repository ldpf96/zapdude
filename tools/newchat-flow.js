(function () {
    var button = document.querySelector('button[aria-label="New chat"]')
        || document.querySelector('button[aria-label*="chat nuevo" i], button[aria-label*="new chat" i]');
    if (!button) {
        return { ok: false, error: 'new_chat_button_not_found' };
    }
    button.click();
    // Small settle so the drawer mounts.
    return new Promise(function (resolve) {
        setTimeout(function () {
            var drawer = document.querySelector('[data-testid="new-chat-drawer"]');
            if (!drawer) {
                resolve({ ok: false, error: 'drawer_not_mounted' });
                return;
            }
            var input = drawer.querySelector('input[aria-label*="Search name" i]')
                || document.querySelector('input[aria-label*="Search name" i]');
            if (!input) {
                resolve({ ok: false, error: 'drawer_input_not_found' });
                return;
            }
            input.focus({ preventScroll: true });
            var setter = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, 'value').set;
            setter.call(input, '+51987931332');
            input.dispatchEvent(new Event('input', { bubbles: true }));
            setTimeout(function () {
                resolve({ ok: true, typed: true, drawerMounted: true });
            }, 900);
        }, 800);
    });
})()
