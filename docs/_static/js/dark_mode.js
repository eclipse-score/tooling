/*
  *******************************************************************************
  Copyright (c) 2026 Contributors to the Eclipse Foundation

  See the NOTICE file(s) distributed with this work for additional
  information regarding copyright ownership.

  This program and the accompanying materials are made available under the
  terms of the Apache License Version 2.0 which is available at
  https://www.apache.org/licenses/LICENSE-2.0

  SPDX-License-Identifier: Apache-2.0
  *******************************************************************************
*/

/**
 * Dark Mode Toggle for Sphinx RTD Theme
 * Provides automatic dark mode detection and manual toggle capability
 */

(function() {
    'use strict';

    const STORAGE_KEY = 'sphinx-rtd-theme-color-mode';
    const DARK_MODE_CLASS = 'dark-mode-enabled';

    function getSystemPrefersDark() {
        return window.matchMedia('(prefers-color-scheme: dark)').matches;
    }

    function applyMode(mode) {
        const useDark = mode === 'dark' || (mode === 'auto' && getSystemPrefersDark());

        if (useDark) {
            document.body.classList.add(DARK_MODE_CLASS);
            document.documentElement.style.colorScheme = 'dark';
        } else {
            // Light mode: remove everything, let RTD theme render as default
            document.body.classList.remove(DARK_MODE_CLASS);
            document.documentElement.style.removeProperty('color-scheme');
        }

        updateToggleButton();
    }

    /**
     * Initialize dark mode based on user preference or saved setting
     */
    function initializeDarkMode() {
        const savedPreference = localStorage.getItem(STORAGE_KEY);

        if (savedPreference === 'dark' || savedPreference === 'light') {
            applyMode(savedPreference);
        } else {
            applyMode('auto');
        }
    }

    /**
     * Set dark mode on or off
     * @param {boolean} isDark - Whether to enable dark mode
     */
    function setDarkMode(isDark) {
        const mode = isDark ? 'dark' : 'light';
        localStorage.setItem(STORAGE_KEY, mode);
        applyMode(mode);
    }

    /**
     * Toggle dark mode
     */
    function toggleDarkMode() {
        const isDarkEnabled = document.body.classList.contains(DARK_MODE_CLASS);
        setDarkMode(!isDarkEnabled);
    }

    /**
     * Update the toggle button to reflect current state
     */
    function updateToggleButton() {
        const toggle = document.getElementById('dark-mode-toggle');
        if (toggle) {
            const isDarkEnabled = document.body.classList.contains(DARK_MODE_CLASS);
            toggle.setAttribute('aria-pressed', isDarkEnabled);
            toggle.title = isDarkEnabled ? 'Switch to light mode' : 'Switch to dark mode';
            toggle.innerHTML = isDarkEnabled ? '☀️' : '🌙';
        }
    }

    /**
     * Create and inject the dark mode toggle button
     */
    function createToggleButton() {
        // Check if toggle already exists
        if (document.getElementById('dark-mode-toggle')) {
            return;
        }

        const toggle = document.createElement('button');
        toggle.id = 'dark-mode-toggle';
        toggle.className = 'dark-mode-toggle';
        toggle.setAttribute('aria-label', 'Toggle dark mode');
        toggle.type = 'button';
        toggle.addEventListener('click', toggleDarkMode);

        // Prefer RTD top nav when visible; fall back to a floating button.
        const navTop = document.querySelector('.wy-nav-top');
        const canUseNavTop = navTop && window.getComputedStyle(navTop).display !== 'none';

        if (canUseNavTop) {
            toggle.style.cssText = `
                position: absolute;
                right: 20px;
                top: 50%;
                transform: translateY(-50%);
                font-size: 20px;
                padding: 8px;
                cursor: pointer;
                background: none;
                border: none;
                color: inherit;
                z-index: 1000;
            `;
            navTop.appendChild(toggle);
        } else {
            // Place inside the sidebar header (next to "Rules Score Doc" home link)
            const sideNavSearch = document.querySelector('.wy-side-nav-search');
            if (sideNavSearch) {
                toggle.style.cssText = `
                    position: absolute;
                    top: 10px;
                    right: 10px;
                    width: 32px;
                    height: 32px;
                    border-radius: 50%;
                    border: none;
                    background: rgba(255, 255, 255, 0.15);
                    color: #ffffff;
                    font-size: 18px;
                    cursor: pointer;
                    z-index: 1000;
                    line-height: 32px;
                    text-align: center;
                    padding: 0;
                `;
                sideNavSearch.style.position = 'relative';
                sideNavSearch.appendChild(toggle);
            } else {
                // Fallback if sidebar not found (e.g. mobile collapsed)
                toggle.style.cssText = `
                    position: fixed;
                    top: 16px;
                    left: 16px;
                    width: 36px;
                    height: 36px;
                    border-radius: 50%;
                    border: 1px solid rgba(120, 120, 120, 0.35);
                    background: rgba(30, 30, 30, 0.9);
                    color: #ffffff;
                    font-size: 18px;
                    cursor: pointer;
                    z-index: 2000;
                    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.25);
                    padding: 0;
                `;
                document.body.appendChild(toggle);
            }
        }

        updateToggleButton();
    }

    /**
     * Listen for system theme changes
     */
    function watchSystemTheme() {
        const darkModeQuery = window.matchMedia('(prefers-color-scheme: dark)');

        // Handle theme changes (for browsers that support this)
        if (darkModeQuery.addEventListener) {
            darkModeQuery.addEventListener('change', (e) => {
                const savedPreference = localStorage.getItem(STORAGE_KEY);

                // Follow system only when no explicit user preference was stored.
                if (savedPreference !== 'dark' && savedPreference !== 'light') {
                    applyMode('auto');
                }
            });
        }
    }

    // Initialize when DOM is ready
    if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', function() {
            initializeDarkMode();
            createToggleButton();
            watchSystemTheme();
        });
    } else {
        initializeDarkMode();
        createToggleButton();
        watchSystemTheme();
    }

    // Expose toggle function globally for manual control
    window.toggleDarkMode = toggleDarkMode;
})();
