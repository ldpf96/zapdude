(function () {
    var input = document.querySelector('input[aria-label="Search name, number or @username"]')
        || document.querySelector('input[aria-label*="Search name" i]');
    if (!input) {
        return { ok: false, error: 'drawer_search_not_found' };
    }
    input.focus({ preventScroll: true });
    var setter = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, 'value').set;
    setter.call(input, '51987931332');
    input.dispatchEvent(new Event('input', { bubbles: true }));
    return { ok: true, typed: true };
})()
