var webpack = require('webpack');
var node_dir = __dirname + '/node_modules';
const MiniCssExtractPlugin = require("mini-css-extract-plugin");

module.exports = {
  entry: './assets/js/main.js',
  plugins: [
    new webpack.ProvidePlugin({
      $: "jquery",
      jQuery: "jquery",
      iziToast: "izitoast",
      DOMPurify: 'dompurify'
    }),
    new MiniCssExtractPlugin({
      filename: "bundle.css"
    })
  ],
  output: {
    path: __dirname + '/static/dist',
    filename: 'bundle.js'
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
}
