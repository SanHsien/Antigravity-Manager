const baseColor = (variable) => `color-mix(in srgb, var(${variable}) calc(100% * <alpha-value>), transparent)`;

/** @type {import('tailwindcss').Config} */
export default {
    content: [
        "./index.html",
        "./src/**/*.{js,ts,jsx,tsx}",
    ],
    theme: {
        extend: {
            colors: {
                'base-100': baseColor('--color-base-100'),
                'base-200': baseColor('--color-base-200'),
                'base-300': baseColor('--color-base-300'),
                'base-content': baseColor('--color-base-content'),
            },
        },
    },
}
