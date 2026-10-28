/**
 * ZapZap (Rust) — hide WhatsApp Web's "download the app" promos.
 *
 * SAFE VERSION. An earlier generic sweep hid any container whose text
 * merely CONTAINED a promo phrase; WhatsApp's landing containers matched
 * too and the whole page went invisible (bare dark background — looked
 * like a "gray page" while the DOM stayed alive).
 *
 * This version can never blank the page:
 *   1. Only the SMALLEST element matching the promo text gets hidden
 *      (containers are skipped when a descendant already matches).
 *   2. Then, walking UP from each hidden node, an ancestor is collapsed
 *      only when EVERY one of its children is display:none (computed) and
 *      it holds the promo — so a big ancestor dies only when all of its
 *      content is already promo content.
 *   3. QR, chat list, composer and any functional element never match the
 *      promo pattern, so they are never candidates.
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

    var CANDIDATE_SELECTOR =
        'div, aside, header, section, span, p, h1, h2, h3, button, a';

    function matches(node) {
        try {
            var text = (node.getAttribute('aria-label') || '') + ' ' + (node.textContent || '');
            return PROMO_PATTERN.test(text.slice(0, 400));
        } catch (_error) {
            return false;
        }
    }

    function isHidden(node) {
        try {
            return node.style && node.style.display === 'none';
        } catch (_error) {
            return false;
        }
    }

    function hide(node) {
        try {
            if (node.style && node.style.display !== 'none') {
                node.style.setProperty('display', 'none', 'important');
                collapseAncestors(node);
            }
        } catch (_error) {
            // Never break the page because of cosmetics.
        }
    }

    /// Hide the chain of ancestors that are left with only hidden children
    /// (bounded climb, computed-style check). Started from a node that just
    /// got hidden — so an ancestor can only die if ALL of its content was
    /// promo content that is already hidden.
    function collapseAncestors(start) {
        var node = start;
        for (var depth = 0; depth < 6; depth++) {
            var parent = node.parentElement;
            if (!parent || parent === document.body || parent === document.documentElement) {
                return;
            }
            var children = parent.children;
            if (children.length === 0 || children.length > 4) {
                return; // keep mid-sized layouts alone
            }
            for (var i = 0; i < children.length; i++) {
                var child = children[i];
                var hidden = false;
                try {
                    hidden = getComputedStyle(child).display === 'none';
                } catch (_error) {
                    return;
                }
                if (!hidden) {
                    return; // something else is still visible — stop climbing
                }
            }
            try {
                parent.style.setProperty('display', 'none', 'important');
            } catch (_error) {
                return;
            }
            node = parent;
        }
    }

    function sweep() {
        try {
            var candidates = document.querySelectorAll(CANDIDATE_SELECTOR);
            for (var i = 0; i < candidates.length; i++) {
                var node = candidates[i];
                if (isHidden(node) || !matches(node)) {
                    continue;
                }
                // Only the SMALLEST matching element dies: skip containers
                // when a descendant already matches the promo pattern.
                var descendants = node.querySelectorAll(CANDIDATE_SELECTOR);
                var smallest = true;
                for (var j = 0; j < descendants.length; j++) {
                    if (matches(descendants[j])) {
                        smallest = false;
                        break;
                    }
                }
                if (smallest) {
                    hide(node);
                }
            }
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
        }, 600);
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
