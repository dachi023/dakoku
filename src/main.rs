mod commands;
mod config;
mod format;
mod paths;
mod store;
mod timearg;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "dakoku",
    version,
    about = "リポジトリごとに作業時間を記録する打刻CLI"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// このリポジトリで作業を開始する
    In {
        /// 作業内容
        note: Option<String>,
        /// 開始時刻を遡って指定する (例: 09:30, "2026-08-30 09:30", -90m)
        #[arg(long, value_name = "TIME", allow_hyphen_values = true)]
        at: Option<String>,
        /// 打刻中のセッションを締めてから開始する
        #[arg(short, long)]
        switch: bool,
    },
    /// 打刻中のセッションを締めて作業時間を表示する
    Out {
        /// 終了時刻を遡って指定する (例: 18:00, "2026-08-30 18:00", -15m)
        #[arg(long, value_name = "TIME", allow_hyphen_values = true)]
        at: Option<String>,
    },
    /// 打刻中のセッションを表示する
    Status,
    /// 記録を一覧する (既定は今日)
    Log {
        /// 今週の月曜以降
        #[arg(long, conflicts_with_all = ["month", "all", "since"])]
        week: bool,
        /// 今月1日以降
        #[arg(long, conflicts_with_all = ["all", "since"])]
        month: bool,
        /// すべての記録
        #[arg(long, conflicts_with = "since")]
        all: bool,
        /// 指定日以降 (例: 2026-08-01)
        #[arg(long, value_name = "DATE")]
        since: Option<String>,
        /// このリポジトリの記録だけに絞る
        #[arg(long)]
        here: bool,
    },
    /// 打刻中、または直前のセッションを修正する
    Edit {
        /// 作業内容を書き換える
        #[arg(long, value_name = "TEXT")]
        note: Option<String>,
        /// 開始時刻を変更する
        #[arg(long = "in", value_name = "TIME", allow_hyphen_values = true)]
        start: Option<String>,
        /// 終了時刻を変更する
        #[arg(long = "out", value_name = "TIME", allow_hyphen_values = true)]
        end: Option<String>,
    },
    /// リポジトリに紐づくラベルを設定・確認する
    Label {
        #[command(subcommand)]
        action: LabelAction,
    },
}

#[derive(Subcommand)]
enum LabelAction {
    /// リポジトリにラベルを設定する
    Set {
        /// リポジトリに付けるラベル
        label: String,
        /// 対象のリポジトリ (既定はカレント)
        #[arg(long, value_name = "PATH")]
        path: Option<String>,
    },
    /// 効いているラベルを表示する
    Show {
        #[arg(long, value_name = "PATH")]
        path: Option<String>,
    },
    /// 設定済みのリポジトリを一覧する
    List,
    /// ラベルの設定を削除する
    Unset {
        #[arg(long, value_name = "PATH")]
        path: Option<String>,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::In { note, at, switch } => commands::clock::clock_in(note, at, switch),
        Command::Out { at } => commands::clock::clock_out(at),
        Command::Status => commands::clock::status(),
        Command::Log {
            week,
            month,
            all,
            since,
            here,
        } => commands::log::run(commands::log::Range { week, month, all }, since, here),
        Command::Edit { note, start, end } => commands::edit::run(note, start, end),
        Command::Label { action } => match action {
            LabelAction::Set { label, path } => commands::label::set(label, path),
            LabelAction::Show { path } => commands::label::show(path),
            LabelAction::List => commands::label::list(),
            LabelAction::Unset { path } => commands::label::unset(path),
        },
    }
}
