var webpack = require('webpack');
var node_dir = __dirname + '/node_modules';
const MiniCssExtractPlugin = require("mini-css-extract-plugin");

module.exports = {
  entry: {
    main: './assets/js/main.js',
    index: './assets/js/index.js',
    agents: './assets/js/agents.js',
    jobs: './assets/js/jobs.js',
    job_page: './assets/js/job_page.js',
    crashes: './assets/js/crashes.js',
    crash_page: './assets/js/crash_page.js'
  },
  plugins: [
    new webpack.ProvidePlugin({
      $: "jquery",
      jQuery: "jquery",
      iziToast: "izitoast",
      DOMPurify: 'dompurify'
    }),
    new MiniCssExtractPlugin({
      filename: "[name].css"
    })
  ],
  output: {
    path: __dirname + '/static/dist',
    filename: '[name].js',
  },
  module: {
    rules: [
      {
        test: /\.css$/i,
        use: [MiniCssExtractPlugin.loader, "css-loader"],
      },
      {
        test: /\.js$/,
        exclude: /node_modules/,
        loader: 'babel-loader',
      },
      {
        test: /\.(png|jpg|gif)$/i,
        type: 'asset/resource'
      }
    ],
  },
  optimization: {
    runtimeChunk: 'single'
  },
}
