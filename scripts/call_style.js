/**
 * Zapdude - brand-styling pass for WhatsApp's in-page call prompt.
 *
 * The incoming-call prompt (ACCEPT/DECLINE bar) is WhatsApp's OWN UI: it
 * cannot be replaced by the branded native toast (it must stay clickable),
 * but it can be BEAUTIFIED. This ships cosmetic CSS ONLY, anchored on the
 * SEMANTIC testids captured live from a real call:
 *   div[data-testid^="voip-container-"]  ... the prompt shell
 *   div[data-testid="voip-call-participant-info-name"] ... caller line
 *   img ... (the caller avatar, https CDN src)
 * plus generic button smoothing inside the shell.
 *
 * HARD RULES (the adopt-without-breaking doctrine):
 *   - NO display/visibility/position changes anywhere: geometry and
 *     functionality always stay WhatsApp's own. Only radii, shadows,
 *     border tones, font weight and color smoothing.
 *   - Every rule is attribute-anchored: if WhatsApp changes its markup,
 *     the whole stylesheet silently stops matching (self-degrade, the
 *     page never breaks) - the same doctrine as the promo hider.
 *   - One idempotency flag, own try/catch, installed deferred.
 */
(function () {
    'use strict';

    if (window !== window.top) {
        return;
    }
    if (window.__zapdudeCallStyleV1) {
        return;
    }
    window.__zapdudeCallStyleV1 = true;

    var css = [
        // The prompt shell: rounded, deep-shadow, brand-dark.
        'div[data-testid^="voip-container-"] {',
        '  border-radius: 16px !important;',
        '  box-shadow: 0 14px 44px rgba(0, 0, 0, 0.5), 0 0 0 1px rgba(255, 255, 255, 0.06) !important;',
        '  background: #0b141a !important;',
        '  overflow: hidden;',
        '}',
        // The caller avatar: soft round.
        'div[data-testid^="voip-container-"] img {',
        '  border-radius: 12px !important;',
        '}',
        // Buttons keep all their behavior; only text weight. Their own
        // pill shape was already pretty (real capture 10:10:32) - forcing
        // a radius over it only deviated from WhatsApp's design.
        'div[data-testid^="voip-container-"] button {',
        '  font-weight: 600 !important;',
        '}',
        // The caller line: crisp brand text.
        'div[data-testid="voip-call-participant-info-name"] * {',
        '  color: #eef4f1 !important;',
        '  font-weight: 600 !important;',
        '  letter-spacing: 0.2px !important;',
        '}',
    ].join('\n');

    function start() {
        try {
            var style = document.createElement('style');
            style.setAttribute('data-zapdude', 'call-style-v1');
            style.textContent = css;
            document.head.appendChild(style);
        } catch (_styleError) {
            // Cosmetic only - never break the page.
        }
    }

    if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', start);
    } else {
        start();
    }
})();
