(function () {
    'use strict';

    // Alignment between the applied user agent and the Chromium Client Hints
    // surfaced to the page. Some services hide feature entry points (notably
    // WhatsApp Web's voice and video call buttons) when the JS-facing identity
    // disagrees with the User-Agent header. This script rebuilds the JS-facing
    // identity before any page script runs; header alignment is managed by the
    // bundled WebView2 runtime identity.

    function chromeMajorVersion() {
        var userAgent = navigator.userAgent || '';
        var match = userAgent.match(/Chrome\/(\d+)/);
        return match ? match[1] : null;
    }

    function detectPlatformName() {
        var userAgent = (navigator.userAgent || '').toLowerCase();
        if (userAgent.indexOf('macintosh') >= 0 || userAgent.indexOf('mac os') >= 0) {
            return 'macOS';
        }
        if (userAgent.indexOf('windows') >= 0) {
            return 'Windows';
        }
        if (userAgent.indexOf('android') >= 0) {
            return 'Android';
        }
        return 'Linux';
    }

    function detectPlatformVersion() {
        var userAgent = (navigator.userAgent || '').toLowerCase();
        var platforms = {
            macos: userAgent.match(/mac os x ([0-9_]+)/),
            windows: userAgent.match(/windows nt ([0-9.]+)/)
        };
        if (platforms.macos) {
            return platforms.macos[1].replace(/_/g, '.');
        }
        if (platforms.windows) {
            return platforms.windows[1];
        }
        return '0.0.0';
    }

    function buildBrands(major) {
        return [
            { brand: 'Not?A_Brand', version: '99' },
            { brand: 'Google Chrome', version: String(major) },
            { brand: 'Chromium', version: String(major) }
        ];
    }

    function buildUserAgentData() {
        var major = chromeMajorVersion();
        var fallbackBrands = [];
        try {
            if (navigator.userAgentData && Array.isArray(navigator.userAgentData.brands)) {
                fallbackBrands = navigator.userAgentData.brands.slice();
            }
        } catch (_ignore) {
            fallbackBrands = [];
        }
        var brands = major ? buildBrands(major) : fallbackBrands;
        var core = {
            brands: brands,
            mobile: /android|iphone/i.test(navigator.userAgent || ''),
            platform: detectPlatformName()
        };
        var highEntropy = {
            architecture: 'x86',
            bitness: '64',
            platformVersion: detectPlatformVersion(),
            uaFullVersion: String(major || 0) + '.0.0.0',
            fullVersionList: brands,
            model: '',
            mobile: core.mobile,
            platform: core.platform
        };
        var data = {
            brands: core.brands,
            mobile: core.mobile,
            platform: core.platform,
            getHighEntropyValues: function (hints) {
                return new Promise(function (resolve, reject) {
                    try {
                        if (!Array.isArray(hints)) {
                            reject(new TypeError('hints must be an array'));
                            return;
                        }
                        var result = {};
                        hints.forEach(function (hintName) {
                            if (Object.prototype.hasOwnProperty.call(highEntropy, hintName)) {
                                result[hintName] = highEntropy[hintName];
                            }
                        });
                        resolve(result);
                    } catch (_error) {
                        reject(_error);
                    }
                });
            },
            toJSON: function () {
                return { brands: core.brands, mobile: core.mobile, platform: core.platform };
            }
        };

        // Freeze so the page cannot mutate the alignment values.
        try {
            Object.freeze(core.brands);
            Object.freeze(data);
        } catch (_ignore) {
            /* ignore */
        }
        return data;
    }

    try {
        var userAgentData = buildUserAgentData();
        var hasData = !!(
            userAgentData
            && userAgentData.brands
            && userAgentData.brands.length
        );
        var needsOverride = true;
        try {
            if (
                navigator.userAgentData
                && Array.isArray(navigator.userAgentData.brands)
                && navigator.userAgentData.brands.some(function (brand) {
                    return (
                        brand
                        && brand.brand === 'Google Chrome'
                        && chromeMajorVersion()
                        && String(brand.version) === String(chromeMajorVersion())
                    );
                })
                && navigator.userAgentData.platform === userAgentData.platform
            ) {
                needsOverride = false;
            }
        } catch (_compareError) {
            needsOverride = true;
        }

        if (hasData && needsOverride) {
            Object.defineProperty(navigator, 'userAgentData', {
                get: function () {
                    return userAgentData;
                },
                configurable: true,
                enumerable: true
            });
        }
    } catch (_setupError) {
        // Never break the page for an identity alignment failure.
    }
})();
