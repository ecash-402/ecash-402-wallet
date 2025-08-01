use std::sync::Arc;

use cdk::nuts::CurrencyUnit;
use cdk::wallet::{ReceiveOptions, Wallet};
use cdk_redb::WalletRedbDatabase;
use rand::random;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let seed = random::<[u8; 32]>();
    let mint_url = "https://ecashmint.otrta.me";

    let unit = CurrencyUnit::Msat;

    let home_dir = home::home_dir().unwrap();
    println!("{:?}", home_dir);
    let localstore = WalletRedbDatabase::new(&home_dir.join("hello")).unwrap();
    println!("{:?}", seed);
    // let localstore = memory::empty().await?;
    let wallet = Wallet::new(mint_url, unit, Arc::new(localstore), &seed, None).unwrap();
    let token = "cashuBo2FteBpodHRwczovL2VjYXNobWludC5vdHJ0YS5tZWF1ZG1zYXRhdIGiYWlIAEdWs6T5owRhcI2kYWEZgABhc3hAMmFmZWE5NDNjZDM5OTY1ODA1NjgxZGJlNzk4MThjYTBiZDBkODljYzJjNzJkZjYzMzliMGU5YTk1N2ZmOGVhN2FjWCECqvpNotmVLpbTENtnSRYlo-gax_tYq7qAOgqs1eqdBG5hZKNhZVggG09G_FPOPqIx9YOdF_YUgW2x84Yh0C3kMcGFxIIX_KVhc1ggZI5YfH5DxYDxDt1KiPP-wFdy3hcZLC_Qky5Hzfen--NhclggYvzaDd50hS-yfIZ_LbLnqp86vP6em-qxdBJKU_nC382kYWEZQABhc3hAYjhjYjA2M2JiM2FlZjFlODY1NjgyMGNlY2RjZWMyMjNlMjVkZmU5MGFlOTEzMDg4NzgwNGZmMmIyZTczYzA0ZmFjWCED76bW73-AqnrLLiJziAeLEzyh3lS3gNan7O1wKWWv8u1hZKNhZVggtW46LmuVewphfDYp0JO54nWThmfP7JXHk2o8-sryriVhc1ggmuYLwkm5Z7l9EBC3xau-wNVOY8SZjc8FIxdUGpeoms1hclggyrli-cZ-jov1xAckAe-v-edV4X7DoVtsWh-wLxGs_PukYWEZIABhc3hANjgyNDhlNmVjN2EwNTJmOTg1ZWMyZWI1NDAwYTUzMWQ2MTA1NjE1YTI2MTAzNGFmY2JjZjk4Yjg2YjRkMmMwZWFjWCEC9d6qO-T97ZHEL1x0SzBxuMup16U3VtZvVpblVf-zJ79hZKNhZVggM7t2SkOuSzQPrFm37O0d3QOZp1ei4rL8WG0mz8vlIW5hc1ggDx8e8BOKp9tm6_CxKGtgVNv5-jtBtBn8T28u4SBmUWdhclggU0ooVinUM97jEXZRPhsd1seGIEK2bcllKDXxeEKmttCkYWEZEABhc3hAMjE1ZWJlMjEzNjdlNTY0MGViMzY0YThmMjQ0MTZjMjVlOTNjMzI0ZjQ4YmQ1MmY0MzZhOGYzMGQzYmEyMTU5Y2FjWCECN3_oOXx-3J9BSqLBWcONFiS5JO6MccDi6qNxypR9j3JhZKNhZVgg_JQYHAbihFpHrT_nQuZ4bfh9X3wpxg8HcdteSjuJA9hhc1ggcdtd_gVhky_emQZIV6BEH2tIEOaxkyMoEmcd1AIC0PBhclggTwVwNG0JC12NfXpxGwAU2lgjiqDXMNg5bNaM9DkGG5-kYWEZCABhc3hAYTYxNWI4NjZhYTEzYWI5MzcxMTdjNjEwOTJhMGJlMzE1ZGQwM2EzNDE1MWQ0NDBlYzlmY2NmZDQ1OGQ4N2ZjZGFjWCED7l3drOnflh6T8_l8fMyUWFUZdTuGHOTZSTQbrdWLb91hZKNhZVggGytI1Aw58nF3-6nvAgxgWmREXX6H9kcIpNORqmHqQG1hc1ggfCAHacPhbr-dbdNGRy0kXaF0ye0sy47kBHYgQdTfmUJhclgg6Q1mUWm7aqcJF_QXAJqu2kvF0xv2AqBR0vCj8idFYRukYWEZAgBhc3hAMTI5ZWYwNTk0Mzg1YjU1YmU5Y2FlZDdjY2YxNTZhYzVhOGI4MDI1ODc0MGMzNTUwMGY2ZjRjNGMzOGQ4MzRkN2FjWCEDaPKzCxunria1BUC3AB3X6b9V0BmhElxVH7uOlmscw2dhZKNhZVggraBjR0NI3tCaXIdPNVS5RRPUWWlOISARDmgS_Pr-iYhhc1ggMPwBfbXg56Q8j4UJ3xXhkMHZ87k5PZ9FayxSCiNhsS9hclgg0rk3LwbwZGWGbBP35fOi6nke-rksWj1tl75cN4Z-C5ekYWEZAQBhc3hAZDJkZDRhNDQxZGQ5ODU5MzRmZTYyMjk0ZWUwMDMyNTNjODUxZDI1Yjc1ZTQ0MDE1MTRjMGI1YjkwODdhZGU4NWFjWCECVFGhBC6mtigis6jEs4NCAWyWVZO-dkXX0WdgYIFk15NhZKNhZVggukaHlO0yniWlDYxhXeA6H3rs7p0YmHSgGKNu8-HVoAJhc1gg2vM80jqVTfprnm89bEl30DdnGqMiS9rlrGBuf-Aenf1hclggoLNvPth1SXl1C3u6n9jNr5qt40MOpquKKGuM2PMPLbKkYWEYgGFzeEAxY2EzODM5ZTkwYTEzMDhkNmYzNWNlZmE2MGIyNzE0MzE5YzhmOTIwMDQ2Njc3ZDBlMTkxMmNlYmEwZTViYzVmYWNYIQOyji-ktsT2K3XNFR-GwK7Qretc8v4r5tSCdapNxB0FpmFko2FlWCBGiMLrOjR9Ekc8VmvH_MZr8cIpRM2B2VpG7czyDtz4mGFzWCCPP_amMS7jQM-DlMXSa2kr4eM9BRgdS3ihdcEoYmca62FyWCDn-Pql4NvlWkJu5XGnsDEKmx3-Ean1Tmlu8AdnYRwCwKRhYRggYXN4QGEyNTYxNDc3ZDU4MDg0MzBjNzU0MTM3OWMyZjZhNTFmMzIwNGNkMThiNjU5NTgyMzlkNzZlMDQ1YThkOGQyY2JhY1ghAx49hCM_MdB_xxqGAwbAZk2AVM4wm43kp-yYIgZq-ieCYWSjYWVYIGuP_US4R5jxJNOMgKzMCBbN8ECs0QAFgqypOCW7IbSkYXNYIPUppcJ8L4VxKPQHlbXlwAaSz9M7pie18NiWDz1v2EKFYXJYIDPHtZnosayDyoM0s4CvrsHUJZjHZ8uLetl_sWG5B9jgpGFhGCBhc3hAYTQxOGRiYmE2ZTJmMWNiYjZkMmQzM2QyMWI3YzIwZmZiZDI5YjVmYjBkZmI1OWJiZjJhOGJiNDRjOGMwN2IzNGFjWCECAfCa1jfrNKzOXJvPe4v6wDJLXmovU_93Dklo52tKp4phZKNhZVggIZJpIWK59MqN_o59_f3ONLrMuk_ioEGJwClMdSQxcpFhc1ggL50wapviZElJeaV6CBDcn8TVnv8rr_ZWwpCLaVDas6phclggH-6WkB6bHmArhNLJigc4B3kxLXGbCfI9PbnhsdPjphOkYWEYIGFzeEBhNWZlNzMyYzY0YWJmNWFmZjlkZTgyNWJkMDc4N2FkM2JjZDI0NjI5ZWIyOTFjNGM3Njc0YThiZTI4NDMzZjAwYWNYIQJI2BssyFo2sLaLvCvKoimzf9YPhr0cFrDFKiloLLiXbGFko2FlWCAD7d20m4qtFlXDm08LDVTuiENNRQ-sqeW9i4NVL6oPcWFzWCB63c1SENQK9VTzCK25f3n6ndBAevN0IN65RbFy2OX3MGFyWCBXQyrQG1jzlPgcwcFSIGAAI91j1ETQ16EqScXB1VcEYKRhYRBhc3hANWM2MzQzNzFmNzAzMjFkMjJlZTJiOGY2NDI0MmQ2NDc1MmQ3OGEwNTZmNmYyNWM2OWQxMjMwYzBiMGU4MmFlOWFjWCECOAnU98Ttwuvv7gXMjp7cPD6O6WyI1EANZxppVjvi-71hZKNhZVgg4RUgJkns1DpDtmUigj3w-3CIGe1V6OeAq7wWtSF5a7Nhc1ggzcfdMRekvTxCs3avxDXOGcrBRKkVyDMa7HfAhiEDFMVhclggLS-O4RIL4ctXKm7LNnTRfxkzrANXqqCEMQcUPqowW6ikYWEEYXN4QDE1MjZjNjRjYjZiZTdiNmU4MWYzNWNjMGJmMDhjODI0Y2YxYTBmMWZlNDAwYmNiNzgwM2E5YjY3MGZjZjdiODNhY1ghAjnt-7y84NLHA00vWXLS12gqO6bmYT07DK1YReWTeL_DYWSjYWVYIFxJiBvN8uqJggy2cguoCe8yF73XZpR6kvEGVfZDNIwnYXNYIGCQQsM1pzK7Ey4ZqViP263x_ijr-CVvapdtXqdxd7iaYXJYIF9eVOkIWWDBZIPKumvagr6C866VBlIfOLxocvpui2d_";
    let amount_receive = wallet.receive(token, ReceiveOptions::default()).await?;
    println!("{}", amount_receive);
    Ok(())
}
