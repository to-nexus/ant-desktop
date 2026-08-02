export default {
  plugins: {
    // v4 moved the PostCSS plugin out of `tailwindcss` into its own package.
    // autoprefixer is gone with it — v4 handles prefixing via Lightning CSS.
    "@tailwindcss/postcss": {},
  },
};
