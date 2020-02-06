const path = require('path');

module.exports = {
    entry: './js/ingest.js',
    output: {
        path: path.resolve(__dirname, 'dist'),
        filename: 'ingest.js'
    },
    module: {
        rules: [
            {
                test: /\.m?js$/,
                exclude: /(node_modules|bower_components)/,
                use: {
                    loader: 'babel-loader',
                    options: {
                        presets: ['@babel/preset-env']
                    }
                }
            }
        ]
    },
    devServer: {
        index: '',
        proxy: {
            context: () => true,
            target: 'http://127.0.0.1:1212',
            bypass: function (req, res, proxyOptions) {
                if (req.url == '/style.css') {
                    return 'src/site/style.css';
                }
            }
        }
    }
};
