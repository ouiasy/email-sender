#!/usr/bin/env bash
# ^ path上のbashを探して実行する
set -x # 実行されたコマンドを出力する
set -eo pipefail
# e: コマンド失敗時終了、-o pipefail: パイプライン内のどれかのコマンドが失敗したら、それをエラーとみなす

sqlx migrate run