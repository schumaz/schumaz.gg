use super::*;

#[test]
fn test_mcp_document_parses_class_headers_nested_tuples_and_empty_lists() {
    let dsl = "class LolGetChampionAnalysis: data\n\
        class Data: summary,core_items,empty_list\n\
        class CoreItems: ids,play,win,pick_rate\n\n\
        LolGetChampionAnalysis(Data(Summary(AverageStats(0.512,0.118)),CoreItems([3118,4645,3157],1200,620,0.42),[]))\"";

    let doc = McpDocument::parse(dsl);
    assert_eq!(doc.fields_for(&["Data"]), Some(&["summary", "core_items", "empty_list"][..]));
    assert_eq!(
        doc.fields_for(&["CoreItems"]),
        Some(&["ids", "play", "win", "pick_rate"][..])
    );

    let data = *doc.find_tuples(&["Data"]).first().expect("Data tuple");
    let named = doc.named_args(data).expect("Data schema arity matches");
    assert_eq!(named[2].0, "empty_list");
    assert_eq!(named[2].1.as_list().map(<[McpValue]>::len), Some(0));

    let core = *doc.find_tuples(&["CoreItems"]).first().expect("CoreItems");
    assert_eq!(
        doc.field(core, "ids").and_then(McpValue::as_i32_list),
        Some(vec![3118, 4645, 3157])
    );
    assert_eq!(doc.field(core, "play").and_then(McpValue::as_i64), Some(1200));
    assert_eq!(doc.field(core, "win").and_then(McpValue::as_i64), Some(620));
    assert_eq!(
        doc.field(core, "pick_rate").and_then(McpValue::as_f64),
        Some(0.42)
    );
}

#[test]
fn test_mcp_document_handles_escaped_quotes_single_quotes_and_scalars() {
    let dsl = r#"Wrap(Item(1,"Dano \"extra\" (15%), 'ok'",'Q',-5,3.5e1,true,false,null,None))"#;
    let doc = McpDocument::parse(dsl);
    let item = *doc.find_tuples(&["Item"]).first().expect("Item tuple");
    assert_eq!(item.args.len(), 9);
    assert_eq!(item.args[0].as_i32(), Some(1));
    assert_eq!(item.args[1].as_str(), Some(r#"Dano "extra" (15%), 'ok'"#));
    assert_eq!(item.args[2].as_str(), Some("Q"));
    assert_eq!(item.args[3].as_i64(), Some(-5));
    assert_eq!(item.args[4].as_f64(), Some(35.0));
    assert_eq!(item.args[5], McpValue::Bool(true));
    assert_eq!(item.args[6], McpValue::Bool(false));
    assert_eq!(item.args[7], McpValue::Null);
    assert_eq!(item.args[8], McpValue::Null);
}

#[test]
fn test_mcp_document_handles_multibyte_utf8_boundaries_and_truncated_eof() {
    let dsl = "Descrição pt_BR — çãé 증강 🔥 \
        \"Augment(9999,\\\"Ignorado dentro de aspas\\\",1,1.0,1.0)\" \
        çAugment(9998,\"Inválido\",3,50.0,0.1) \
        NãoAugment(9997,\"Prefixo\",3,50.0,0.1) \
        —Augment(1074,\"Dano çãé em área\",3,79.754,0.224) \
        🔥Augment(2062,\"증강 설명 (15%)\",4,91.2,0.15) \
        Texto final çãé — Augment(3075,\"Incompleto çãé";

    let doc = McpDocument::parse(dsl);
    let augments = doc.find_tuples(&["Augment"]);
    assert_eq!(augments.len(), 2);
    assert_eq!(augments[0].args[0].as_i32(), Some(1074));
    assert_eq!(augments[0].args[1].as_str(), Some("Dano çãé em área"));
    assert_eq!(augments[1].args[0].as_i32(), Some(2062));
    assert_eq!(augments[1].args[1].as_str(), Some("증강 설명 (15%)"));
}
