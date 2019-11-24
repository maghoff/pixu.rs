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
    }
};
