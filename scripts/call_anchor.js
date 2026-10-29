/**
 * Zapdude call anchor - passive discovery phase.
 *
 * WhatsApp Web's call surface (the emulated popup shown for in-page calls)
 * is draggable, which holds it captive inside the app box: the user wants
 * it either FIXED at its spot or, for the popup variant, freely movable.
 * Before pinning anything down, the exact markup of the call surface must
 * be observed (WhatsApp redesigns it between releases and a guess-based
 * script once broke startup).
 *
 * Phase 1 (this file): MutationObserver that records every voip/call
 * surface element whenever one appears, and ships the structure to the
 * host through the bridge (chat-list beacon transport). Harmless: it never
 * modifies the DOM aliens.
 *
 * Phase 2 (future edits here once the data lands): pin the surface to its
 * default position (drop the drag) and add the popup-close recovery.
 */
(function () {
    'use strict';

    if (window.__zapdudeCallAnchor) {
        return;
    }
    window.__zapdudeCallAnchor = true;

    var last_shipped = 0;

    function isCallSurface(node) {
        try {
            if (node.nodeType !== 1) {
                return false;
            }
            var testid = String(node.getAttribute && node.getAttribute('data-testid') || '');
            if (/voip|call/i.test(testid) && !/log|bubble|icce/i.test(testid)) {
                var rect = node.getBoundingClientRect();
                return rect.width > 120 && rect.height > 120;
            }
            var aria = String((node.getAttribute && node.getAttribute('aria-label')) || '');
            return /current call|llamada actual|llamada en curso/i.test(aria)
                && rect.width > 120 && rect.height > 120;
        } catch (_inspectError) {
            return false;
        }
    }

    function describe(node) {
        var rect = { width: 0, height: 0, left: 0, top: 0 };
        var aria = '';
        var testid = '';
        var cls = '';
        var buttons = [];
        var transform = '';
        try {
            var bounds = node.getBoundingClientRect();
            rect = {
                left: Math.round(bounds.left),
                top: Math.round(bounds.top),
                width: Math.round(bounds.width),
                height: Math.round(bounds.height),
            };
            aria = String(node.getAttribute('aria-label') || '');
            testid = String(node.getAttribute('data-testid') || '');
            cls = String(node.className || '').slice(0, 90);
            transform = String(node.style.transform || '');
            node.querySelectorAll('button, [role="button"]').forEach(function (button) {
                var label = String(button.getAttribute('aria-label') || '');
                var icon = '';
                var iconNode = button.querySelector('span[data-icon]');
                if (iconNode) {
                    icon = String(iconNode.getAttribute('data-icon') || '');
                }
                if (label || icon) {
                    buttons.push({ label: label.slice(0, 40), icon: icon });
                }
            });
        } catch (_describeError) {}
        return {
            rect: rect,
            aria: aria.slice(0, 60),
            testid: testid,
            cls: cls,
            transform: transform,
            buttons: buttons.slice(0, 12),
        };
    }

    function scan() {
        try {
            var findings = [];
            var candidates = document.querySelectorAll('[data-testid], [aria-label]');
            for (var index = 0; index < candidates.length; index++) {
                var node = candidates[index];
                if (!isCallSurface(node)) {
                    continue;
                }
                if (node.contains && Array.prototype.some.call(findings, function (found) {
                    return found.contains(node);
                })) {
                    continue; // only the outermost surface ships
                }
                findings.push(node);
            }
            if (!findings.length) {
                return;
            }
            if (Date.now() - last_shipped < 4000) {
                return; // debounce repeated captures of the same surface
            }
            last_shipped = Date.now();
            var surfaces = findings.map(describe);
            var payload = {
                kind: 'call_surface',
                booted: true,
                surfaces: surfaces,
            };
            try {
                if (window.__zapdudeApiV1 && typeof window.__zapdudeApiV1.pushEvent === 'function') {
                    window.__zapdudeApiV1.pushEvent(JSON.stringify(payload));
                }
            } catch (_bridgeError) {}
        } catch (_scanError) {}
    }

    var scheduled = null;
    function scheduleScan() {
        if (scheduled) {
            return;
        }
        scheduled = setTimeout(function () {
            scheduled = null;
            scan();
        }, 350);
    }

    function start() {
        scan();
        try {
            var observer = new MutationObserver(scheduleScan);
            observer.observe(document.documentElement, {
                childList: true,
                subtree: true,
                attributes: false,
            });
        } catch (_observerError) {}
    }

    if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', start);
    } else {
        start();
    }
})();
