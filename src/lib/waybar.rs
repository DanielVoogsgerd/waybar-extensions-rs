use std::iter::once;

use serde::Serialize;

#[derive(Serialize)]
pub struct WaybarResponse {
    pub text: String,
    pub tooltip: String,
    pub class: Vec<String>,
}

pub fn columnize_output(output: &[Vec<String>], column_heading: &[String]) -> String {
    let heading_size = column_heading.iter().map(|x| x.len()).collect::<Vec<_>>();
    let max_size = output
        .iter()
        .fold(heading_size, |mut acc: Vec<usize>, cur| {
            acc.iter_mut()
                .zip(cur.iter())
                .for_each(|(acc_val, cur_val)| {
                    if cur_val.len() > *acc_val {
                        std::mem::swap(acc_val, &mut cur_val.len());
                    }
                });

            acc
        });

    once(
        column_heading
            .iter()
            .enumerate()
            .map(|(i, val)| format!("<b>{val:width$}</b>", width = max_size[i]))
            .collect::<Vec<_>>()
            .join("    "),
    )
    .chain(output.iter().map(|row| {
        row.iter()
            .enumerate()
            .map(|(i, val)| format!("{val:width$}", width = max_size[i]))
            .collect::<Vec<_>>()
            .join("    ")
    }))
    .collect::<Vec<_>>()
    .join("\n")
}
