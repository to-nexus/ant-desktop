/** @type {import('tailwindcss').Config} */
export default {
  content: ["./index.html", "./src/**/*.{js,ts,jsx,tsx}"],
  theme: {
    extend: {
      colors: {
        status: {
          connected: "#22c55e",
          warning: "#f59e0b",
          error: "#ef4444",
          inactive: "#9ca3af",
        },
      },
    },
  },
  plugins: [],
};
