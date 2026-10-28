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
            'available\\s+for\\s+windows',
            'obten\\w*\\s+funciones\\s+adicionales',
            'funciones\\s+adicionales',
            'get\\s+extra\\s+features',
            'extra\\s+features\\s+like'
        ].join('|'),
        'i'
    );

    /// Standalone promo buttons ("Descargar", "Download") — exact-text only,
    /// never a substring, so image/file download buttons elsewhere survive.
    var BUTTON_PROMO_PATTERN = /^\s*(descarga\w*|download|obtener|get|instala\w*|install)\s*$/i;

    /// WhatsApp's inline SVG icons carry a <title> like "ic-download" that
    /// textContent concatenates onto the label ("Downloadic-download").
    /// Strip those tokens (and collapse whitespace) before exact-text tests.
    function cleanText(raw) {
        return String(raw || '')
            .replace(/ic-[\w-]+/g, ' ')
            .replace(/\s+/g, ' ')
            .trim();
    }

    var CANDIDATE_SELECTOR =
        'div, aside, header, section, span, p, h1, h2, h3, button, a';

    /// Chat/composer territory is off-limits: nothing inside these gets
    /// hidden, whatever it says.
    function inProtectedZone(node) {
        try {
            return !!node.closest(
                '#pane-side, div[role="listitem"], [contenteditable="true"], ' +
                '[contenteditable], #main, div[role="textbox"], canvas, input, textarea'
            );
        } catch (_error) {
            return true; // when in doubt, protect
        }
    }

    function matches(node) {
        try {
            if (inProtectedZone(node)) {
                return false;
            }
            var own = (node.getAttribute('aria-label') || '') + ' ' + (node.textContent || '');
            own = own.slice(0, 400);
            if (PROMO_PATTERN.test(own)) {
                return true;
            }
            // Button/link whose whole wording is just "Descargar"/"Download"
            // (with or without an icon inside): promo furniture on the
            // landing page. In-chat media download buttons live inside the
            // protected zone above, so they can never reach this check.
            var tag = node.tagName;
            if (tag === 'BUTTON' || tag === 'A') {
                var label = cleanText(node.getAttribute('aria-label'));
                var text = cleanText(node.textContent);
                var buttonish = (label.length <= 12 && BUTTON_PROMO_PATTERN.test(label)) ||
                    (text.length <= 12 && BUTTON_PROMO_PATTERN.test(text));
                if (buttonish) {
                    return true;
                }
            }
            return false;
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

    /// True when at least one matching element is actually RENDERED (an
    /// element with no client rects is display:none/hidden).
    function anyVisible(selector) {
        try {
            var els = document.querySelectorAll(selector);
            for (var i = 0; i < els.length; i++) {
                if (els[i].getClientRects().length > 0) {
                    return true;
                }
            }
        } catch (_error) {
            return true; // when in doubt, assume the UI is there
        }
        return false;
    }

    /// Is this child acceptable inside a promo container? Hidden nodes,
    /// promo matches, pure icons and bare "Download"-style buttons qualify.
    function childIsPromoish(child) {
        if (isHidden(child)) {
            return true;
        }
        try {
            var own = cleanText(child.textContent);
            // A button/link whose whole wording is "Download"/"Descargar"
            // (icon + short label) is promo furniture, even with an svg.
            if (own.length <= 12 && BUTTON_PROMO_PATTERN.test(own)) {
                var tag = child.tagName;
                if (tag === 'BUTTON' || tag === 'A' || tag === 'SPAN' || tag === 'DIV') {
                    return true;
                }
            }
            if ((child.tagName === 'IMG' || child.tagName === 'PICTURE' ||
                 child.tagName === 'CANVAS' || child.tagName === 'svg' ||
                 child.querySelector('svg, img, canvas, picture')) &&
                own.length === 0) {
                return true; // pure icon / illustration
            }
        } catch (_error) {
            return false;
        }
        return matches(child);
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

    /// Hide the chain of ancestors that are left with only promo-ish children
    /// (bounded climb). Started from a node that just got hidden — so an
    /// ancestor can only die if EVERY one of its children is already hidden,
    /// an icon, or promo content. A functional control (composer, chat list)
    /// can never be promo-ish, so real UI is untouchable.
    function collapseAncestors(start) {
        var node = start;
        for (var depth = 0; depth < 8; depth++) {
            var parent = node.parentElement;
            if (!parent || parent === document.body || parent === document.documentElement) {
                return;
            }
            var children = parent.children;
            if (children.length === 0 || children.length > 6) {
                return; // keep mid-sized layouts alone
            }
            for (var i = 0; i < children.length; i++) {
                if (!childIsPromoish(children[i])) {
                    return; // real content — stop climbing
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
            hideLandingPromoCard();
        } catch (_error) {
            // Never break the page because of cosmetics.
        }
    }

    /// Landing-page promo card ("Download WhatsApp for Windows" next to the
    /// QR). Only runs on the LOGGED-OUT landing (no chat list, no chat pane):
    /// find the control that just says "Download"/"Descargar" and climb to
    /// the SMALLEST ancestor that also contains promo copy — that is the
    /// whole card (illustration + copy + button). Safety rails: any
    /// functional element (canvas/QR, inputs, editables) or a text budget
    /// blowout stops the climb, so the QR card can never be matched.
    function hideLandingPromoCard() {
        // Visibility-based guard: WhatsApp's React shell may keep a HIDDEN
        // #main/#pane-side in the DOM even on the logged-out landing, so a
        // mere existence check would disable this flow forever.
        if (anyVisible('#pane-side, #main')) {
            return; // logged-in UI: the generic sweep already handles promos
        }
        var buttons = document.querySelectorAll('button, a, [role="button"]');
        for (var i = 0; i < buttons.length; i++) {
            var btn = buttons[i];
            // NOTE: hidden buttons are NOT skipped — the generic sweep hides
            // the bare "Download" button first, and that hidden button is the
            // trail this flow follows up to the enclosing promo card.
            if (inProtectedZone(btn)) {
                continue;
            }
            var text = cleanText(btn.textContent);
            if (text.length > 12 || !BUTTON_PROMO_PATTERN.test(text)) {
                continue;
            }
            var card = null;
            var node = btn;
            for (var depth = 0; depth < 8 && node; depth++) {
                node = node.parentElement;
                if (!node || node === document.body || node === document.documentElement) {
                    break;
                }
                if ((node.textContent || '').trim().length > 400) {
                    break; // text budget: this is a layout, not a card
                }
                if (PROMO_PATTERN.test(node.textContent || '')) {
                    card = node; // smallest enclosing element with promo copy
                    break;
                }
            }
            hide(card || btn);
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
