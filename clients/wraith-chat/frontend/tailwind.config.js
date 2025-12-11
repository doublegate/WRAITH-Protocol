/** @type {import('tailwindcss').Config} */
export default {
  content: ['./index.html', './src/**/*.{js,ts,jsx,tsx}'],
  theme: {
    extend: {
      colors: {
        wraith: {
          primary: '#6366f1',
          secondary: '#8b5cf6',
          accent: '#ec4899',
          dark: '#1e1b4b',
          darker: '#0f0d2e',
        },
      },
    },
  },
  plugins: [],
};
