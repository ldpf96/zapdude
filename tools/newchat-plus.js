(function () {
    var input = document.querySelector('input[aria-label="Search name, number or @username"]');
    if (!input) {
        return { ok: false, error: 'no_input' };
    }
    input.focus({ preventScroll: true });
    var setter = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, 'value').set;
    setter.call(input, '+51987931332');
    input.dispatchEvent(new Event('input', { bubbles: true }));
    return { ok: true, typed: '+51987931332' };
})()
