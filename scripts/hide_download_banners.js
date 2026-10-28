/**
 * ZapZap (Rust) — hide WhatsApp Web's "download the app" promos.
 *
 * WhatsApp Web keeps pushing banners, dialogs and toasts that recommend
 * installing the official Windows app. Inside ZapZap those recommendations
 * are noise (the user already runs a desktop client), so this script hides
 * them defensively:
 *   - matches by text (ES/EN) on dialogs, banners, toasts and landing cards;
 *   - survives WhatsApp markup changes (MutationObserver + periodic sweep);
 *   - never touches the QR code, chat UI or any functional element:
 *     elements are hidden with display:none only when the promo pattern
 *     matches, so a false positive costs at most one banner.
 */
(function () {
    'use strict';

    if (window !== window.top) {
        return; // top frame only
    }
    if (window.__zapdudeHidePromos) {
        return; // already installed
    }
    window.__zapdudeHidePromos = true;

    var PROMO_PATTERN = new RegExp(
        [
            'descarga\\w*\\s+(la\\s+aplicaci|whatsapp)',
            'download\\s+(the\\s+app|whatsapp)',
            'obtener\\s+la\\s+(aplicaci|app)',
            'get\\s+the\\s+app',
            'instala\\w*\\s+(la\\s+aplicaci|whatsapp)',
            'install\\s+(the\\s+app|whatsapp)',
            'usar\\s+whatsapp\\s+en\\s+tu',
            'use\\s+whatsapp\\s+on\\s+your',
            'whatsapp\\s+para\\s+windows',
            'whatsapp\\s+for\\s+windows',
            'disponible\\s+para\\s+windows',
            'available\\s+for\\s+windows'
        ].join('|'),
        'i'
    );

    // Selectors known to host download promos on the WhatsApp Web landing
    // screen (safe list: they only ever contain the promo itself).
    var LANDED_PROMO_SELECTORS = [
        '[data-zapdude-promo]',
        'div[data-animate-modal-backdrop]',
        '[role="dialog"][aria-modal="true"]'
    ].join(', ');

    function elementText(node) {
        var text = '';
        if (node.getAttribute) {
            text += ' ' + (node.getAttribute('aria-label') || '');
        }
        try {
            text += ' ' + (node.textContent || '');
        } catch (_error) {
            return '';
        }
        return text.slice(0, 400);
    }

    function looksLikePromo(node) {
        if (!node || node.nodeType !== 1) {
            return false;
        }
        var tag = node.tagName;
        if (tag !== 'DIV' && tag !== 'ASIDE' && tag !== 'HEADER' && tag !== 'SECTION') {
            return false;
        }
        return PROMO_PATTERN.test(elementText(node));
    }

    function hide(node) {
        if (node.style && node.style.display !== 'none') {
            node.style.setProperty('display', 'none', 'important');
        }
    }

    function sweep() {
        try {
            // Landing-screen promos and modal backdrops are hidden outright.
            document.querySelectorAll(LANDED_PROMO_SELECTORS).forEach(function (node) {
                if (looksLikePromo(node)) {
                    hide(node);
                }
            });
            // Toast-style promos live in generic containers; match by text.
            document.querySelectorAll('div, aside, header, section').forEach(function (node) {
                if (!node.__zapdudeChecked) {
                    node.__zapdudeChecked = true;
                    if (looksLikePromo(node) && node.childElementCount <= 6) {
                        hide(node);
                    }
                }
            });
        } catch (_error) {
            // Never break the page because of cosmetics.
        }
    }

    var scheduled = null;
    function scheduleSweep() {
        if (scheduled) {
            return;
        }
        scheduled = setTimeout(function () {
            scheduled = null;
            sweep();
        }, 400);
    }

    function start() {
        sweep();
        var observer = new MutationObserver(scheduleSweep);
        observer.observe(document.documentElement, { childList: true, subtree: true });
    }

    if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', start);
    } else {
        start();
    }
})();
