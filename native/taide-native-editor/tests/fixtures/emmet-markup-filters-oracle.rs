const CASES: &[(&str, &str, Option<&str>)] = &[
    (
        "html",
        "div.block|bem",
        Some("<div class=\"block\">${0}</div>"),
    ),
    (
        "html",
        "div.block|c",
        Some("<div class=\"block\">${0}</div>\n<!-- /.block -->"),
    ),
    (
        "html",
        "div.block|t",
        Some("<div class=\"block\">${0}</div>"),
    ),
    (
        "html",
        "div.block|bem|c",
        Some("<div class=\"block\">${0}</div>\n<!-- /.block -->"),
    ),
    (
        "html",
        "div.block|c|bem",
        Some("<div class=\"block\">${0}</div>\n<!-- /.block -->"),
    ),
    (
        "html",
        "div.block|bem|t|c",
        Some("<div class=\"block\">${0}</div>\n<!-- /.block -->"),
    ),
    (
        "html",
        "div.block|c|c|c",
        Some("<div class=\"block\">${0}</div>\n<!-- /.block -->"),
    ),
    (
        "html",
        "div.block|bem|bem",
        Some("<div class=\"block\">${0}</div>"),
    ),
    (
        "html",
        "div.block|c|t",
        Some("<div class=\"block\">${0}</div>\n<!-- /.block -->"),
    ),
    (
        "html",
        "div.block|t|c|bem",
        Some("<div class=\"block\">${0}</div>\n<!-- /.block -->"),
    ),
    (
        "html",
        "div.block_mod|bem",
        Some("<div class=\"block block_mod\">${0}</div>"),
    ),
    (
        "html",
        "div.block_mod|c",
        Some("<div class=\"block_mod\">${0}</div>\n<!-- /.block_mod -->"),
    ),
    (
        "html",
        "div.block_mod|t",
        Some("<div class=\"block_mod\">${0}</div>"),
    ),
    (
        "html",
        "div.block_mod|bem|c",
        Some("<div class=\"block block_mod\">${0}</div>\n<!-- /.block block_mod -->"),
    ),
    (
        "html",
        "div.block_mod|c|bem",
        Some("<div class=\"block block_mod\">${0}</div>\n<!-- /.block block_mod -->"),
    ),
    (
        "html",
        "div.block_mod|bem|t|c",
        Some("<div class=\"block block_mod\">${0}</div>\n<!-- /.block block_mod -->"),
    ),
    (
        "html",
        "div.block_mod|c|c|c",
        Some("<div class=\"block_mod\">${0}</div>\n<!-- /.block_mod -->"),
    ),
    (
        "html",
        "div.block_mod|bem|bem",
        Some("<div class=\"block block_mod\">${0}</div>"),
    ),
    (
        "html",
        "div.block_mod|c|t",
        Some("<div class=\"block_mod\">${0}</div>\n<!-- /.block_mod -->"),
    ),
    (
        "html",
        "div.block_mod|t|c|bem",
        Some("<div class=\"block block_mod\">${0}</div>\n<!-- /.block block_mod -->"),
    ),
    (
        "html",
        "div.block__element_mod|bem",
        Some("<div class=\"block block_element_mod\">${0}</div>"),
    ),
    (
        "html",
        "div.block__element_mod|c",
        Some("<div class=\"block__element_mod\">${0}</div>\n<!-- /.block__element_mod -->"),
    ),
    (
        "html",
        "div.block__element_mod|t",
        Some("<div class=\"block__element_mod\">${0}</div>"),
    ),
    (
        "html",
        "div.block__element_mod|bem|c",
        Some(
            "<div class=\"block block_element_mod\">${0}</div>\n<!-- /.block block_element_mod -->",
        ),
    ),
    (
        "html",
        "div.block__element_mod|c|bem",
        Some(
            "<div class=\"block block_element_mod\">${0}</div>\n<!-- /.block block_element_mod -->",
        ),
    ),
    (
        "html",
        "div.block__element_mod|bem|t|c",
        Some(
            "<div class=\"block block_element_mod\">${0}</div>\n<!-- /.block block_element_mod -->",
        ),
    ),
    (
        "html",
        "div.block__element_mod|c|c|c",
        Some("<div class=\"block__element_mod\">${0}</div>\n<!-- /.block__element_mod -->"),
    ),
    (
        "html",
        "div.block__element_mod|bem|bem",
        Some("<div class=\"block block_element_mod\">${0}</div>"),
    ),
    (
        "html",
        "div.block__element_mod|c|t",
        Some("<div class=\"block__element_mod\">${0}</div>\n<!-- /.block__element_mod -->"),
    ),
    (
        "html",
        "div.block__element_mod|t|c|bem",
        Some(
            "<div class=\"block block_element_mod\">${0}</div>\n<!-- /.block block_element_mod -->",
        ),
    ),
    (
        "html",
        "div.b-block.other|bem",
        Some("<div class=\"b-block other\">${0}</div>"),
    ),
    (
        "html",
        "div.b-block.other|c",
        Some("<div class=\"b-block other\">${0}</div>\n<!-- /.b-block other -->"),
    ),
    (
        "html",
        "div.b-block.other|t",
        Some("<div class=\"b-block other\">${0}</div>"),
    ),
    (
        "html",
        "div.b-block.other|bem|c",
        Some("<div class=\"b-block other\">${0}</div>\n<!-- /.b-block other -->"),
    ),
    (
        "html",
        "div.b-block.other|c|bem",
        Some("<div class=\"b-block other\">${0}</div>\n<!-- /.b-block other -->"),
    ),
    (
        "html",
        "div.b-block.other|bem|t|c",
        Some("<div class=\"b-block other\">${0}</div>\n<!-- /.b-block other -->"),
    ),
    (
        "html",
        "div.b-block.other|c|c|c",
        Some("<div class=\"b-block other\">${0}</div>\n<!-- /.b-block other -->"),
    ),
    (
        "html",
        "div.b-block.other|bem|bem",
        Some("<div class=\"b-block other\">${0}</div>"),
    ),
    (
        "html",
        "div.b-block.other|c|t",
        Some("<div class=\"b-block other\">${0}</div>\n<!-- /.b-block other -->"),
    ),
    (
        "html",
        "div.b-block.other|t|c|bem",
        Some("<div class=\"b-block other\">${0}</div>\n<!-- /.b-block other -->"),
    ),
    (
        "html",
        "div.-element|bem",
        Some("<div class=\"__element\">${0}</div>"),
    ),
    (
        "html",
        "div.-element|c",
        Some("<div class=\"-element\">${0}</div>\n<!-- /.-element -->"),
    ),
    (
        "html",
        "div.-element|t",
        Some("<div class=\"-element\">${0}</div>"),
    ),
    (
        "html",
        "div.-element|bem|c",
        Some("<div class=\"__element\">${0}</div>\n<!-- /.__element -->"),
    ),
    (
        "html",
        "div.-element|c|bem",
        Some("<div class=\"__element\">${0}</div>\n<!-- /.__element -->"),
    ),
    (
        "html",
        "div.-element|bem|t|c",
        Some("<div class=\"__element\">${0}</div>\n<!-- /.__element -->"),
    ),
    (
        "html",
        "div.-element|c|c|c",
        Some("<div class=\"-element\">${0}</div>\n<!-- /.-element -->"),
    ),
    (
        "html",
        "div.-element|bem|bem",
        Some("<div class=\"__element\">${0}</div>"),
    ),
    (
        "html",
        "div.-element|c|t",
        Some("<div class=\"-element\">${0}</div>\n<!-- /.-element -->"),
    ),
    (
        "html",
        "div.-element|t|c|bem",
        Some("<div class=\"__element\">${0}</div>\n<!-- /.__element -->"),
    ),
    (
        "html",
        "div._modifier|bem",
        Some("<div class=\"_modifier\">${0}</div>"),
    ),
    (
        "html",
        "div._modifier|c",
        Some("<div class=\"_modifier\">${0}</div>\n<!-- /._modifier -->"),
    ),
    (
        "html",
        "div._modifier|t",
        Some("<div class=\"_modifier\">${0}</div>"),
    ),
    (
        "html",
        "div._modifier|bem|c",
        Some("<div class=\"_modifier\">${0}</div>\n<!-- /._modifier -->"),
    ),
    (
        "html",
        "div._modifier|c|bem",
        Some("<div class=\"_modifier\">${0}</div>\n<!-- /._modifier -->"),
    ),
    (
        "html",
        "div._modifier|bem|t|c",
        Some("<div class=\"_modifier\">${0}</div>\n<!-- /._modifier -->"),
    ),
    (
        "html",
        "div._modifier|c|c|c",
        Some("<div class=\"_modifier\">${0}</div>\n<!-- /._modifier -->"),
    ),
    (
        "html",
        "div._modifier|bem|bem",
        Some("<div class=\"_modifier\">${0}</div>"),
    ),
    (
        "html",
        "div._modifier|c|t",
        Some("<div class=\"_modifier\">${0}</div>\n<!-- /._modifier -->"),
    ),
    (
        "html",
        "div._modifier|t|c|bem",
        Some("<div class=\"_modifier\">${0}</div>\n<!-- /._modifier -->"),
    ),
    (
        "html",
        "div.block>.-element|bem",
        Some("<div class=\"block\">\n\t<div class=\"block__element\">${0}</div>\n</div>"),
    ),
    (
        "html",
        "div.block>.-element|c",
        Some(
            "<div class=\"block\">\n\t<div class=\"-element\">${0}</div>\n\t<!-- /.-element -->\n</div>\n<!-- /.block -->",
        ),
    ),
    (
        "html",
        "div.block>.-element|t",
        Some("<div class=\"block\">\n\t<div class=\"-element\">${0}</div>\n</div>"),
    ),
    (
        "html",
        "div.block>.-element|bem|c",
        Some(
            "<div class=\"block\">\n\t<div class=\"block__element\">${0}</div>\n\t<!-- /.block__element -->\n</div>\n<!-- /.block -->",
        ),
    ),
    (
        "html",
        "div.block>.-element|c|bem",
        Some(
            "<div class=\"block\">\n\t<div class=\"block__element\">${0}</div>\n\t<!-- /.block__element -->\n</div>\n<!-- /.block -->",
        ),
    ),
    (
        "html",
        "div.block>.-element|bem|t|c",
        Some(
            "<div class=\"block\">\n\t<div class=\"block__element\">${0}</div>\n\t<!-- /.block__element -->\n</div>\n<!-- /.block -->",
        ),
    ),
    (
        "html",
        "div.block>.-element|c|c|c",
        Some(
            "<div class=\"block\">\n\t<div class=\"-element\">${0}</div>\n\t<!-- /.-element -->\n</div>\n<!-- /.block -->",
        ),
    ),
    (
        "html",
        "div.block>.-element|bem|bem",
        Some("<div class=\"block\">\n\t<div class=\"block__element\">${0}</div>\n</div>"),
    ),
    (
        "html",
        "div.block>.-element|c|t",
        Some(
            "<div class=\"block\">\n\t<div class=\"-element\">${0}</div>\n\t<!-- /.-element -->\n</div>\n<!-- /.block -->",
        ),
    ),
    (
        "html",
        "div.block>.-element|t|c|bem",
        Some(
            "<div class=\"block\">\n\t<div class=\"block__element\">${0}</div>\n\t<!-- /.block__element -->\n</div>\n<!-- /.block -->",
        ),
    ),
    (
        "html",
        "div.block>.-element_modifier|bem",
        Some(
            "<div class=\"block\">\n\t<div class=\"block__element block__element_modifier\">${0}</div>\n</div>",
        ),
    ),
    (
        "html",
        "div.block>.-element_modifier|c",
        Some(
            "<div class=\"block\">\n\t<div class=\"-element_modifier\">${0}</div>\n\t<!-- /.-element_modifier -->\n</div>\n<!-- /.block -->",
        ),
    ),
    (
        "html",
        "div.block>.-element_modifier|t",
        Some("<div class=\"block\">\n\t<div class=\"-element_modifier\">${0}</div>\n</div>"),
    ),
    (
        "html",
        "div.block>.-element_modifier|bem|c",
        Some(
            "<div class=\"block\">\n\t<div class=\"block__element block__element_modifier\">${0}</div>\n\t<!-- /.block__element block__element_modifier -->\n</div>\n<!-- /.block -->",
        ),
    ),
    (
        "html",
        "div.block>.-element_modifier|c|bem",
        Some(
            "<div class=\"block\">\n\t<div class=\"block__element block__element_modifier\">${0}</div>\n\t<!-- /.block__element block__element_modifier -->\n</div>\n<!-- /.block -->",
        ),
    ),
    (
        "html",
        "div.block>.-element_modifier|bem|t|c",
        Some(
            "<div class=\"block\">\n\t<div class=\"block__element block__element_modifier\">${0}</div>\n\t<!-- /.block__element block__element_modifier -->\n</div>\n<!-- /.block -->",
        ),
    ),
    (
        "html",
        "div.block>.-element_modifier|c|c|c",
        Some(
            "<div class=\"block\">\n\t<div class=\"-element_modifier\">${0}</div>\n\t<!-- /.-element_modifier -->\n</div>\n<!-- /.block -->",
        ),
    ),
    (
        "html",
        "div.block>.-element_modifier|bem|bem",
        Some(
            "<div class=\"block\">\n\t<div class=\"block__element block__element_modifier\">${0}</div>\n</div>",
        ),
    ),
    (
        "html",
        "div.block>.-element_modifier|c|t",
        Some(
            "<div class=\"block\">\n\t<div class=\"-element_modifier\">${0}</div>\n\t<!-- /.-element_modifier -->\n</div>\n<!-- /.block -->",
        ),
    ),
    (
        "html",
        "div.block>.-element_modifier|t|c|bem",
        Some(
            "<div class=\"block\">\n\t<div class=\"block__element block__element_modifier\">${0}</div>\n\t<!-- /.block__element block__element_modifier -->\n</div>\n<!-- /.block -->",
        ),
    ),
    (
        "html",
        "div.block>.-element>._modifier|bem",
        Some(
            "<div class=\"block\">\n\t<div class=\"block__element\">\n\t\t<div class=\"block block_modifier\">${0}</div>\n\t</div>\n</div>",
        ),
    ),
    (
        "html",
        "div.block>.-element>._modifier|c",
        Some(
            "<div class=\"block\">\n\t<div class=\"-element\">\n\t\t<div class=\"_modifier\">${0}</div>\n\t\t<!-- /._modifier -->\n\t</div>\n\t<!-- /.-element -->\n</div>\n<!-- /.block -->",
        ),
    ),
    (
        "html",
        "div.block>.-element>._modifier|t",
        Some(
            "<div class=\"block\">\n\t<div class=\"-element\">\n\t\t<div class=\"_modifier\">${0}</div>\n\t</div>\n</div>",
        ),
    ),
    (
        "html",
        "div.block>.-element>._modifier|bem|c",
        Some(
            "<div class=\"block\">\n\t<div class=\"block__element\">\n\t\t<div class=\"block block_modifier\">${0}</div>\n\t\t<!-- /.block block_modifier -->\n\t</div>\n\t<!-- /.block__element -->\n</div>\n<!-- /.block -->",
        ),
    ),
    (
        "html",
        "div.block>.-element>._modifier|c|bem",
        Some(
            "<div class=\"block\">\n\t<div class=\"block__element\">\n\t\t<div class=\"block block_modifier\">${0}</div>\n\t\t<!-- /.block block_modifier -->\n\t</div>\n\t<!-- /.block__element -->\n</div>\n<!-- /.block -->",
        ),
    ),
    (
        "html",
        "div.block>.-element>._modifier|bem|t|c",
        Some(
            "<div class=\"block\">\n\t<div class=\"block__element\">\n\t\t<div class=\"block block_modifier\">${0}</div>\n\t\t<!-- /.block block_modifier -->\n\t</div>\n\t<!-- /.block__element -->\n</div>\n<!-- /.block -->",
        ),
    ),
    (
        "html",
        "div.block>.-element>._modifier|c|c|c",
        Some(
            "<div class=\"block\">\n\t<div class=\"-element\">\n\t\t<div class=\"_modifier\">${0}</div>\n\t\t<!-- /._modifier -->\n\t</div>\n\t<!-- /.-element -->\n</div>\n<!-- /.block -->",
        ),
    ),
    (
        "html",
        "div.block>.-element>._modifier|bem|bem",
        Some(
            "<div class=\"block\">\n\t<div class=\"block__element\">\n\t\t<div class=\"block block_modifier\">${0}</div>\n\t</div>\n</div>",
        ),
    ),
    (
        "html",
        "div.block>.-element>._modifier|c|t",
        Some(
            "<div class=\"block\">\n\t<div class=\"-element\">\n\t\t<div class=\"_modifier\">${0}</div>\n\t\t<!-- /._modifier -->\n\t</div>\n\t<!-- /.-element -->\n</div>\n<!-- /.block -->",
        ),
    ),
    (
        "html",
        "div.block>.-element>._modifier|t|c|bem",
        Some(
            "<div class=\"block\">\n\t<div class=\"block__element\">\n\t\t<div class=\"block block_modifier\">${0}</div>\n\t\t<!-- /.block block_modifier -->\n\t</div>\n\t<!-- /.block__element -->\n</div>\n<!-- /.block -->",
        ),
    ),
    (
        "html",
        "div.block>.-element>.__modifier|bem",
        Some(
            "<div class=\"block\">\n\t<div class=\"block__element\">\n\t\t<div class=\"block block_modifier\">${0}</div>\n\t</div>\n</div>",
        ),
    ),
    (
        "html",
        "div.block>.-element>.__modifier|c",
        Some(
            "<div class=\"block\">\n\t<div class=\"-element\">\n\t\t<div class=\"__modifier\">${0}</div>\n\t\t<!-- /.__modifier -->\n\t</div>\n\t<!-- /.-element -->\n</div>\n<!-- /.block -->",
        ),
    ),
    (
        "html",
        "div.block>.-element>.__modifier|t",
        Some(
            "<div class=\"block\">\n\t<div class=\"-element\">\n\t\t<div class=\"__modifier\">${0}</div>\n\t</div>\n</div>",
        ),
    ),
    (
        "html",
        "div.block>.-element>.__modifier|bem|c",
        Some(
            "<div class=\"block\">\n\t<div class=\"block__element\">\n\t\t<div class=\"block block_modifier\">${0}</div>\n\t\t<!-- /.block block_modifier -->\n\t</div>\n\t<!-- /.block__element -->\n</div>\n<!-- /.block -->",
        ),
    ),
    (
        "html",
        "div.block>.-element>.__modifier|c|bem",
        Some(
            "<div class=\"block\">\n\t<div class=\"block__element\">\n\t\t<div class=\"block block_modifier\">${0}</div>\n\t\t<!-- /.block block_modifier -->\n\t</div>\n\t<!-- /.block__element -->\n</div>\n<!-- /.block -->",
        ),
    ),
    (
        "html",
        "div.block>.-element>.__modifier|bem|t|c",
        Some(
            "<div class=\"block\">\n\t<div class=\"block__element\">\n\t\t<div class=\"block block_modifier\">${0}</div>\n\t\t<!-- /.block block_modifier -->\n\t</div>\n\t<!-- /.block__element -->\n</div>\n<!-- /.block -->",
        ),
    ),
    (
        "html",
        "div.block>.-element>.__modifier|c|c|c",
        Some(
            "<div class=\"block\">\n\t<div class=\"-element\">\n\t\t<div class=\"__modifier\">${0}</div>\n\t\t<!-- /.__modifier -->\n\t</div>\n\t<!-- /.-element -->\n</div>\n<!-- /.block -->",
        ),
    ),
    (
        "html",
        "div.block>.-element>.__modifier|bem|bem",
        Some(
            "<div class=\"block\">\n\t<div class=\"block__element\">\n\t\t<div class=\"block block_modifier\">${0}</div>\n\t</div>\n</div>",
        ),
    ),
    (
        "html",
        "div.block>.-element>.__modifier|c|t",
        Some(
            "<div class=\"block\">\n\t<div class=\"-element\">\n\t\t<div class=\"__modifier\">${0}</div>\n\t\t<!-- /.__modifier -->\n\t</div>\n\t<!-- /.-element -->\n</div>\n<!-- /.block -->",
        ),
    ),
    (
        "html",
        "div.block>.-element>.__modifier|t|c|bem",
        Some(
            "<div class=\"block\">\n\t<div class=\"block__element\">\n\t\t<div class=\"block block_modifier\">${0}</div>\n\t\t<!-- /.block block_modifier -->\n\t</div>\n\t<!-- /.block__element -->\n</div>\n<!-- /.block -->",
        ),
    ),
    (
        "html",
        "div.outer>div.inner>.--element_modifier|bem",
        Some(
            "<div class=\"outer\">\n\t<div class=\"inner\">\n\t\t<div class=\"inner__element inner__element_modifier\">${0}</div>\n\t</div>\n</div>",
        ),
    ),
    (
        "html",
        "div.outer>div.inner>.--element_modifier|c",
        Some(
            "<div class=\"outer\">\n\t<div class=\"inner\">\n\t\t<div class=\"--element_modifier\">${0}</div>\n\t\t<!-- /.--element_modifier -->\n\t</div>\n\t<!-- /.inner -->\n</div>\n<!-- /.outer -->",
        ),
    ),
    (
        "html",
        "div.outer>div.inner>.--element_modifier|t",
        Some(
            "<div class=\"outer\">\n\t<div class=\"inner\">\n\t\t<div class=\"--element_modifier\">${0}</div>\n\t</div>\n</div>",
        ),
    ),
    (
        "html",
        "div.outer>div.inner>.--element_modifier|bem|c",
        Some(
            "<div class=\"outer\">\n\t<div class=\"inner\">\n\t\t<div class=\"inner__element inner__element_modifier\">${0}</div>\n\t\t<!-- /.inner__element inner__element_modifier -->\n\t</div>\n\t<!-- /.inner -->\n</div>\n<!-- /.outer -->",
        ),
    ),
    (
        "html",
        "div.outer>div.inner>.--element_modifier|c|bem",
        Some(
            "<div class=\"outer\">\n\t<div class=\"inner\">\n\t\t<div class=\"inner__element inner__element_modifier\">${0}</div>\n\t\t<!-- /.inner__element inner__element_modifier -->\n\t</div>\n\t<!-- /.inner -->\n</div>\n<!-- /.outer -->",
        ),
    ),
    (
        "html",
        "div.outer>div.inner>.--element_modifier|bem|t|c",
        Some(
            "<div class=\"outer\">\n\t<div class=\"inner\">\n\t\t<div class=\"inner__element inner__element_modifier\">${0}</div>\n\t\t<!-- /.inner__element inner__element_modifier -->\n\t</div>\n\t<!-- /.inner -->\n</div>\n<!-- /.outer -->",
        ),
    ),
    (
        "html",
        "div.outer>div.inner>.--element_modifier|c|c|c",
        Some(
            "<div class=\"outer\">\n\t<div class=\"inner\">\n\t\t<div class=\"--element_modifier\">${0}</div>\n\t\t<!-- /.--element_modifier -->\n\t</div>\n\t<!-- /.inner -->\n</div>\n<!-- /.outer -->",
        ),
    ),
    (
        "html",
        "div.outer>div.inner>.--element_modifier|bem|bem",
        Some(
            "<div class=\"outer\">\n\t<div class=\"inner\">\n\t\t<div class=\"inner__element inner__element_modifier\">${0}</div>\n\t</div>\n</div>",
        ),
    ),
    (
        "html",
        "div.outer>div.inner>.--element_modifier|c|t",
        Some(
            "<div class=\"outer\">\n\t<div class=\"inner\">\n\t\t<div class=\"--element_modifier\">${0}</div>\n\t\t<!-- /.--element_modifier -->\n\t</div>\n\t<!-- /.inner -->\n</div>\n<!-- /.outer -->",
        ),
    ),
    (
        "html",
        "div.outer>div.inner>.--element_modifier|t|c|bem",
        Some(
            "<div class=\"outer\">\n\t<div class=\"inner\">\n\t\t<div class=\"inner__element inner__element_modifier\">${0}</div>\n\t\t<!-- /.inner__element inner__element_modifier -->\n\t</div>\n\t<!-- /.inner -->\n</div>\n<!-- /.outer -->",
        ),
    ),
    (
        "html",
        "div.outer>div.inner>.__modifier|bem",
        Some(
            "<div class=\"outer\">\n\t<div class=\"inner\">\n\t\t<div class=\"inner inner_modifier\">${0}</div>\n\t</div>\n</div>",
        ),
    ),
    (
        "html",
        "div.outer>div.inner>.__modifier|c",
        Some(
            "<div class=\"outer\">\n\t<div class=\"inner\">\n\t\t<div class=\"__modifier\">${0}</div>\n\t\t<!-- /.__modifier -->\n\t</div>\n\t<!-- /.inner -->\n</div>\n<!-- /.outer -->",
        ),
    ),
    (
        "html",
        "div.outer>div.inner>.__modifier|t",
        Some(
            "<div class=\"outer\">\n\t<div class=\"inner\">\n\t\t<div class=\"__modifier\">${0}</div>\n\t</div>\n</div>",
        ),
    ),
    (
        "html",
        "div.outer>div.inner>.__modifier|bem|c",
        Some(
            "<div class=\"outer\">\n\t<div class=\"inner\">\n\t\t<div class=\"inner inner_modifier\">${0}</div>\n\t\t<!-- /.inner inner_modifier -->\n\t</div>\n\t<!-- /.inner -->\n</div>\n<!-- /.outer -->",
        ),
    ),
    (
        "html",
        "div.outer>div.inner>.__modifier|c|bem",
        Some(
            "<div class=\"outer\">\n\t<div class=\"inner\">\n\t\t<div class=\"inner inner_modifier\">${0}</div>\n\t\t<!-- /.inner inner_modifier -->\n\t</div>\n\t<!-- /.inner -->\n</div>\n<!-- /.outer -->",
        ),
    ),
    (
        "html",
        "div.outer>div.inner>.__modifier|bem|t|c",
        Some(
            "<div class=\"outer\">\n\t<div class=\"inner\">\n\t\t<div class=\"inner inner_modifier\">${0}</div>\n\t\t<!-- /.inner inner_modifier -->\n\t</div>\n\t<!-- /.inner -->\n</div>\n<!-- /.outer -->",
        ),
    ),
    (
        "html",
        "div.outer>div.inner>.__modifier|c|c|c",
        Some(
            "<div class=\"outer\">\n\t<div class=\"inner\">\n\t\t<div class=\"__modifier\">${0}</div>\n\t\t<!-- /.__modifier -->\n\t</div>\n\t<!-- /.inner -->\n</div>\n<!-- /.outer -->",
        ),
    ),
    (
        "html",
        "div.outer>div.inner>.__modifier|bem|bem",
        Some(
            "<div class=\"outer\">\n\t<div class=\"inner\">\n\t\t<div class=\"inner inner_modifier\">${0}</div>\n\t</div>\n</div>",
        ),
    ),
    (
        "html",
        "div.outer>div.inner>.__modifier|c|t",
        Some(
            "<div class=\"outer\">\n\t<div class=\"inner\">\n\t\t<div class=\"__modifier\">${0}</div>\n\t\t<!-- /.__modifier -->\n\t</div>\n\t<!-- /.inner -->\n</div>\n<!-- /.outer -->",
        ),
    ),
    (
        "html",
        "div.outer>div.inner>.__modifier|t|c|bem",
        Some(
            "<div class=\"outer\">\n\t<div class=\"inner\">\n\t\t<div class=\"inner inner_modifier\">${0}</div>\n\t\t<!-- /.inner inner_modifier -->\n\t</div>\n\t<!-- /.inner -->\n</div>\n<!-- /.outer -->",
        ),
    ),
    (
        "html",
        "div.block.-element_modifier|bem",
        Some("<div class=\"block block__element block__element_modifier\">${0}</div>"),
    ),
    (
        "html",
        "div.block.-element_modifier|c",
        Some(
            "<div class=\"block -element_modifier\">${0}</div>\n<!-- /.block -element_modifier -->",
        ),
    ),
    (
        "html",
        "div.block.-element_modifier|t",
        Some("<div class=\"block -element_modifier\">${0}</div>"),
    ),
    (
        "html",
        "div.block.-element_modifier|bem|c",
        Some(
            "<div class=\"block block__element block__element_modifier\">${0}</div>\n<!-- /.block block__element block__element_modifier -->",
        ),
    ),
    (
        "html",
        "div.block.-element_modifier|c|bem",
        Some(
            "<div class=\"block block__element block__element_modifier\">${0}</div>\n<!-- /.block block__element block__element_modifier -->",
        ),
    ),
    (
        "html",
        "div.block.-element_modifier|bem|t|c",
        Some(
            "<div class=\"block block__element block__element_modifier\">${0}</div>\n<!-- /.block block__element block__element_modifier -->",
        ),
    ),
    (
        "html",
        "div.block.-element_modifier|c|c|c",
        Some(
            "<div class=\"block -element_modifier\">${0}</div>\n<!-- /.block -element_modifier -->",
        ),
    ),
    (
        "html",
        "div.block.-element_modifier|bem|bem",
        Some("<div class=\"block block__element block__element_modifier\">${0}</div>"),
    ),
    (
        "html",
        "div.block.-element_modifier|c|t",
        Some(
            "<div class=\"block -element_modifier\">${0}</div>\n<!-- /.block -element_modifier -->",
        ),
    ),
    (
        "html",
        "div.block.-element_modifier|t|c|bem",
        Some(
            "<div class=\"block block__element block__element_modifier\">${0}</div>\n<!-- /.block block__element block__element_modifier -->",
        ),
    ),
    (
        "html",
        "div.block.foo_bar|bem",
        Some("<div class=\"block foo block_bar\">${0}</div>"),
    ),
    (
        "html",
        "div.block.foo_bar|c",
        Some("<div class=\"block foo_bar\">${0}</div>\n<!-- /.block foo_bar -->"),
    ),
    (
        "html",
        "div.block.foo_bar|t",
        Some("<div class=\"block foo_bar\">${0}</div>"),
    ),
    (
        "html",
        "div.block.foo_bar|bem|c",
        Some("<div class=\"block foo block_bar\">${0}</div>\n<!-- /.block foo block_bar -->"),
    ),
    (
        "html",
        "div.block.foo_bar|c|bem",
        Some("<div class=\"block foo block_bar\">${0}</div>\n<!-- /.block foo block_bar -->"),
    ),
    (
        "html",
        "div.block.foo_bar|bem|t|c",
        Some("<div class=\"block foo block_bar\">${0}</div>\n<!-- /.block foo block_bar -->"),
    ),
    (
        "html",
        "div.block.foo_bar|c|c|c",
        Some("<div class=\"block foo_bar\">${0}</div>\n<!-- /.block foo_bar -->"),
    ),
    (
        "html",
        "div.block.foo_bar|bem|bem",
        Some("<div class=\"block foo block_bar\">${0}</div>"),
    ),
    (
        "html",
        "div.block.foo_bar|c|t",
        Some("<div class=\"block foo_bar\">${0}</div>\n<!-- /.block foo_bar -->"),
    ),
    (
        "html",
        "div.block.foo_bar|t|c|bem",
        Some("<div class=\"block foo block_bar\">${0}</div>\n<!-- /.block foo block_bar -->"),
    ),
    (
        "html",
        "div.block[class=other]|bem",
        Some("<div class=\"block other\">${0}</div>"),
    ),
    (
        "html",
        "div.block[class=other]|c",
        Some("<div class=\"block other\">${0}</div>\n<!-- /.block other -->"),
    ),
    (
        "html",
        "div.block[class=other]|t",
        Some("<div class=\"block other\">${0}</div>"),
    ),
    (
        "html",
        "div.block[class=other]|bem|c",
        Some("<div class=\"block other\">${0}</div>\n<!-- /.block other -->"),
    ),
    (
        "html",
        "div.block[class=other]|c|bem",
        Some("<div class=\"block other\">${0}</div>\n<!-- /.block other -->"),
    ),
    (
        "html",
        "div.block[class=other]|bem|t|c",
        Some("<div class=\"block other\">${0}</div>\n<!-- /.block other -->"),
    ),
    (
        "html",
        "div.block[class=other]|c|c|c",
        Some("<div class=\"block other\">${0}</div>\n<!-- /.block other -->"),
    ),
    (
        "html",
        "div.block[class=other]|bem|bem",
        Some("<div class=\"block other\">${0}</div>"),
    ),
    (
        "html",
        "div.block[class=other]|c|t",
        Some("<div class=\"block other\">${0}</div>\n<!-- /.block other -->"),
    ),
    (
        "html",
        "div.block[class=other]|t|c|bem",
        Some("<div class=\"block other\">${0}</div>\n<!-- /.block other -->"),
    ),
    (
        "html",
        "div.block[class=\"foo bar\"]|bem",
        Some("<div class=\"block foo bar\">${0}</div>"),
    ),
    (
        "html",
        "div.block[class=\"foo bar\"]|c",
        Some("<div class=\"block foo bar\">${0}</div>\n<!-- /.block foo bar -->"),
    ),
    (
        "html",
        "div.block[class=\"foo bar\"]|t",
        Some("<div class=\"block foo bar\">${0}</div>"),
    ),
    (
        "html",
        "div.block[class=\"foo bar\"]|bem|c",
        Some("<div class=\"block foo bar\">${0}</div>\n<!-- /.block foo bar -->"),
    ),
    (
        "html",
        "div.block[class=\"foo bar\"]|c|bem",
        Some("<div class=\"block foo bar\">${0}</div>\n<!-- /.block foo bar -->"),
    ),
    (
        "html",
        "div.block[class=\"foo bar\"]|bem|t|c",
        Some("<div class=\"block foo bar\">${0}</div>\n<!-- /.block foo bar -->"),
    ),
    (
        "html",
        "div.block[class=\"foo bar\"]|c|c|c",
        Some("<div class=\"block foo bar\">${0}</div>\n<!-- /.block foo bar -->"),
    ),
    (
        "html",
        "div.block[class=\"foo bar\"]|bem|bem",
        Some("<div class=\"block foo bar\">${0}</div>"),
    ),
    (
        "html",
        "div.block[class=\"foo bar\"]|c|t",
        Some("<div class=\"block foo bar\">${0}</div>\n<!-- /.block foo bar -->"),
    ),
    (
        "html",
        "div.block[class=\"foo bar\"]|t|c|bem",
        Some("<div class=\"block foo bar\">${0}</div>\n<!-- /.block foo bar -->"),
    ),
    (
        "html",
        "div[class=\"${1:foo}\"]|bem",
        Some("<div class=\"foo\">${0}</div>"),
    ),
    (
        "html",
        "div[class=\"${1:foo}\"]|c",
        Some("<div class=\"${2:foo}\">${3}</div>\n<!-- /.${5:foo} -->"),
    ),
    (
        "html",
        "div[class=\"${1:foo}\"]|t",
        Some("<div class=\"${2:foo}\">${0}</div>"),
    ),
    (
        "html",
        "div[class=\"${1:foo}\"]|bem|c",
        Some("<div class=\"foo\">${0}</div>\n<!-- /.foo -->"),
    ),
    (
        "html",
        "div[class=\"${1:foo}\"]|c|bem",
        Some("<div class=\"foo\">${0}</div>\n<!-- /.foo -->"),
    ),
    (
        "html",
        "div[class=\"${1:foo}\"]|bem|t|c",
        Some("<div class=\"foo\">${0}</div>\n<!-- /.foo -->"),
    ),
    (
        "html",
        "div[class=\"${1:foo}\"]|c|c|c",
        Some("<div class=\"${2:foo}\">${3}</div>\n<!-- /.${5:foo} -->"),
    ),
    (
        "html",
        "div[class=\"${1:foo}\"]|bem|bem",
        Some("<div class=\"foo\">${0}</div>"),
    ),
    (
        "html",
        "div[class=\"${1:foo}\"]|c|t",
        Some("<div class=\"${2:foo}\">${3}</div>\n<!-- /.${5:foo} -->"),
    ),
    (
        "html",
        "div[class=\"${1:foo}\"]|t|c|bem",
        Some("<div class=\"foo\">${0}</div>\n<!-- /.foo -->"),
    ),
    (
        "html",
        "div[class=\"${1}\"]|bem",
        Some("<div class=\"${2}\">${0}</div>"),
    ),
    (
        "html",
        "div[class=\"${1}\"]|c",
        Some("<div class=\"${2}\">${3}</div>\n<!-- /.${0} -->"),
    ),
    (
        "html",
        "div[class=\"${1}\"]|t",
        Some("<div class=\"${2}\">${0}</div>"),
    ),
    (
        "html",
        "div[class=\"${1}\"]|bem|c",
        Some("<div class=\"${2}\">${3}</div>\n<!-- /.${0} -->"),
    ),
    (
        "html",
        "div[class=\"${1}\"]|c|bem",
        Some("<div class=\"${2}\">${3}</div>\n<!-- /.${0} -->"),
    ),
    (
        "html",
        "div[class=\"${1}\"]|bem|t|c",
        Some("<div class=\"${2}\">${3}</div>\n<!-- /.${0} -->"),
    ),
    (
        "html",
        "div[class=\"${1}\"]|c|c|c",
        Some("<div class=\"${2}\">${3}</div>\n<!-- /.${0} -->"),
    ),
    (
        "html",
        "div[class=\"${1}\"]|bem|bem",
        Some("<div class=\"${2}\">${0}</div>"),
    ),
    (
        "html",
        "div[class=\"${1}\"]|c|t",
        Some("<div class=\"${2}\">${3}</div>\n<!-- /.${0} -->"),
    ),
    (
        "html",
        "div[class=\"${1}\"]|t|c|bem",
        Some("<div class=\"${2}\">${3}</div>\n<!-- /.${0} -->"),
    ),
    (
        "html",
        "div.block[!class]|bem",
        Some("<div class=\"block\">${0}</div>"),
    ),
    (
        "html",
        "div.block[!class]|c",
        Some("<div class=\"block\">${0}</div>\n<!-- /.block -->"),
    ),
    (
        "html",
        "div.block[!class]|t",
        Some("<div class=\"block\">${0}</div>"),
    ),
    (
        "html",
        "div.block[!class]|bem|c",
        Some("<div class=\"block\">${0}</div>\n<!-- /.block -->"),
    ),
    (
        "html",
        "div.block[!class]|c|bem",
        Some("<div class=\"block\">${0}</div>\n<!-- /.block -->"),
    ),
    (
        "html",
        "div.block[!class]|bem|t|c",
        Some("<div class=\"block\">${0}</div>\n<!-- /.block -->"),
    ),
    (
        "html",
        "div.block[!class]|c|c|c",
        Some("<div class=\"block\">${0}</div>\n<!-- /.block -->"),
    ),
    (
        "html",
        "div.block[!class]|bem|bem",
        Some("<div class=\"block\">${0}</div>"),
    ),
    (
        "html",
        "div.block[!class]|c|t",
        Some("<div class=\"block\">${0}</div>\n<!-- /.block -->"),
    ),
    (
        "html",
        "div.block[!class]|t|c|bem",
        Some("<div class=\"block\">${0}</div>\n<!-- /.block -->"),
    ),
    (
        "html",
        "div#main.foo|bem",
        Some("<div id=\"main\" class=\"foo\">${0}</div>"),
    ),
    (
        "html",
        "div#main.foo|c",
        Some("<div id=\"main\" class=\"foo\">${0}</div>\n<!-- /#main.foo -->"),
    ),
    (
        "html",
        "div#main.foo|t",
        Some("<div id=\"main\" class=\"foo\">${0}</div>"),
    ),
    (
        "html",
        "div#main.foo|bem|c",
        Some("<div id=\"main\" class=\"foo\">${0}</div>\n<!-- /#main.foo -->"),
    ),
    (
        "html",
        "div#main.foo|c|bem",
        Some("<div id=\"main\" class=\"foo\">${0}</div>\n<!-- /#main.foo -->"),
    ),
    (
        "html",
        "div#main.foo|bem|t|c",
        Some("<div id=\"main\" class=\"foo\">${0}</div>\n<!-- /#main.foo -->"),
    ),
    (
        "html",
        "div#main.foo|c|c|c",
        Some("<div id=\"main\" class=\"foo\">${0}</div>\n<!-- /#main.foo -->"),
    ),
    (
        "html",
        "div#main.foo|bem|bem",
        Some("<div id=\"main\" class=\"foo\">${0}</div>"),
    ),
    (
        "html",
        "div#main.foo|c|t",
        Some("<div id=\"main\" class=\"foo\">${0}</div>\n<!-- /#main.foo -->"),
    ),
    (
        "html",
        "div#main.foo|t|c|bem",
        Some("<div id=\"main\" class=\"foo\">${0}</div>\n<!-- /#main.foo -->"),
    ),
    (
        "html",
        "div#main[ID=upper]|bem",
        Some("<div id=\"main\" ID=\"upper\">${0}</div>"),
    ),
    (
        "html",
        "div#main[ID=upper]|c",
        Some("<div id=\"main\" ID=\"upper\">${0}</div>\n<!-- /#upper -->"),
    ),
    (
        "html",
        "div#main[ID=upper]|t",
        Some("<div id=\"main\" ID=\"upper\">${0}</div>"),
    ),
    (
        "html",
        "div#main[ID=upper]|bem|c",
        Some("<div id=\"main\" ID=\"upper\">${0}</div>\n<!-- /#upper -->"),
    ),
    (
        "html",
        "div#main[ID=upper]|c|bem",
        Some("<div id=\"main\" ID=\"upper\">${0}</div>\n<!-- /#upper -->"),
    ),
    (
        "html",
        "div#main[ID=upper]|bem|t|c",
        Some("<div id=\"main\" ID=\"upper\">${0}</div>\n<!-- /#upper -->"),
    ),
    (
        "html",
        "div#main[ID=upper]|c|c|c",
        Some("<div id=\"main\" ID=\"upper\">${0}</div>\n<!-- /#upper -->"),
    ),
    (
        "html",
        "div#main[ID=upper]|bem|bem",
        Some("<div id=\"main\" ID=\"upper\">${0}</div>"),
    ),
    (
        "html",
        "div#main[ID=upper]|c|t",
        Some("<div id=\"main\" ID=\"upper\">${0}</div>\n<!-- /#upper -->"),
    ),
    (
        "html",
        "div#main[ID=upper]|t|c|bem",
        Some("<div id=\"main\" ID=\"upper\">${0}</div>\n<!-- /#upper -->"),
    ),
    (
        "html",
        "div#main[ID]|bem",
        Some("<div id=\"main\" ID=\"${1}\">${0}</div>"),
    ),
    (
        "html",
        "div#main[ID]|c",
        Some("<div id=\"main\" ID=\"${1}\">${0}</div>\n<!-- /#main -->"),
    ),
    (
        "html",
        "div#main[ID]|t",
        Some("<div id=\"main\" ID=\"${1}\">${0}</div>"),
    ),
    (
        "html",
        "div#main[ID]|bem|c",
        Some("<div id=\"main\" ID=\"${1}\">${0}</div>\n<!-- /#main -->"),
    ),
    (
        "html",
        "div#main[ID]|c|bem",
        Some("<div id=\"main\" ID=\"${1}\">${0}</div>\n<!-- /#main -->"),
    ),
    (
        "html",
        "div#main[ID]|bem|t|c",
        Some("<div id=\"main\" ID=\"${1}\">${0}</div>\n<!-- /#main -->"),
    ),
    (
        "html",
        "div#main[ID]|c|c|c",
        Some("<div id=\"main\" ID=\"${1}\">${0}</div>\n<!-- /#main -->"),
    ),
    (
        "html",
        "div#main[ID]|bem|bem",
        Some("<div id=\"main\" ID=\"${1}\">${0}</div>"),
    ),
    (
        "html",
        "div#main[ID]|c|t",
        Some("<div id=\"main\" ID=\"${1}\">${0}</div>\n<!-- /#main -->"),
    ),
    (
        "html",
        "div#main[ID]|t|c|bem",
        Some("<div id=\"main\" ID=\"${1}\">${0}</div>\n<!-- /#main -->"),
    ),
    (
        "html",
        "div.ID[class=\"\"]|bem",
        Some("<div class=\"ID\">${0}</div>"),
    ),
    (
        "html",
        "div.ID[class=\"\"]|c",
        Some("<div class=\"ID \">${0}</div>\n<!-- /.ID  -->"),
    ),
    (
        "html",
        "div.ID[class=\"\"]|t",
        Some("<div class=\"ID \">${0}</div>"),
    ),
    (
        "html",
        "div.ID[class=\"\"]|bem|c",
        Some("<div class=\"ID\">${0}</div>\n<!-- /.ID -->"),
    ),
    (
        "html",
        "div.ID[class=\"\"]|c|bem",
        Some("<div class=\"ID\">${0}</div>\n<!-- /.ID -->"),
    ),
    (
        "html",
        "div.ID[class=\"\"]|bem|t|c",
        Some("<div class=\"ID\">${0}</div>\n<!-- /.ID -->"),
    ),
    (
        "html",
        "div.ID[class=\"\"]|c|c|c",
        Some("<div class=\"ID \">${0}</div>\n<!-- /.ID  -->"),
    ),
    (
        "html",
        "div.ID[class=\"\"]|bem|bem",
        Some("<div class=\"ID\">${0}</div>"),
    ),
    (
        "html",
        "div.ID[class=\"\"]|c|t",
        Some("<div class=\"ID \">${0}</div>\n<!-- /.ID  -->"),
    ),
    (
        "html",
        "div.ID[class=\"\"]|t|c|bem",
        Some("<div class=\"ID\">${0}</div>\n<!-- /.ID -->"),
    ),
    (
        "html",
        "div.[id=\"\"]|bem",
        Some("<div class=\"${1}\" id=\"\">${0}</div>"),
    ),
    (
        "html",
        "div.[id=\"\"]|c",
        Some("<div class=\"${1}\" id=\"\">${0}</div>\n<!-- /# -->"),
    ),
    (
        "html",
        "div.[id=\"\"]|t",
        Some("<div class=\"${1}\" id=\"\">${0}</div>"),
    ),
    (
        "html",
        "div.[id=\"\"]|bem|c",
        Some("<div class=\"${1}\" id=\"\">${0}</div>\n<!-- /# -->"),
    ),
    (
        "html",
        "div.[id=\"\"]|c|bem",
        Some("<div class=\"${1}\" id=\"\">${0}</div>\n<!-- /# -->"),
    ),
    (
        "html",
        "div.[id=\"\"]|bem|t|c",
        Some("<div class=\"${1}\" id=\"\">${0}</div>\n<!-- /# -->"),
    ),
    (
        "html",
        "div.[id=\"\"]|c|c|c",
        Some("<div class=\"${1}\" id=\"\">${0}</div>\n<!-- /# -->"),
    ),
    (
        "html",
        "div.[id=\"\"]|bem|bem",
        Some("<div class=\"${1}\" id=\"\">${0}</div>"),
    ),
    (
        "html",
        "div.[id=\"\"]|c|t",
        Some("<div class=\"${1}\" id=\"\">${0}</div>\n<!-- /# -->"),
    ),
    (
        "html",
        "div.[id=\"\"]|t|c|bem",
        Some("<div class=\"${1}\" id=\"\">${0}</div>\n<!-- /# -->"),
    ),
    (
        "html",
        "input#main.foo|bem",
        Some("<input type=\"${2:text}\" id=\"main\" class=\"foo\">"),
    ),
    (
        "html",
        "input#main.foo|c",
        Some("<input type=\"${2:text}\" id=\"main\" class=\"foo\">"),
    ),
    (
        "html",
        "input#main.foo|t",
        Some("<input type=\"${2:text}\" id=\"main\" class=\"foo\">"),
    ),
    (
        "html",
        "input#main.foo|bem|c",
        Some("<input type=\"${2:text}\" id=\"main\" class=\"foo\">"),
    ),
    (
        "html",
        "input#main.foo|c|bem",
        Some("<input type=\"${2:text}\" id=\"main\" class=\"foo\">"),
    ),
    (
        "html",
        "input#main.foo|bem|t|c",
        Some("<input type=\"${2:text}\" id=\"main\" class=\"foo\">"),
    ),
    (
        "html",
        "input#main.foo|c|c|c",
        Some("<input type=\"${2:text}\" id=\"main\" class=\"foo\">"),
    ),
    (
        "html",
        "input#main.foo|bem|bem",
        Some("<input type=\"${2:text}\" id=\"main\" class=\"foo\">"),
    ),
    (
        "html",
        "input#main.foo|c|t",
        Some("<input type=\"${2:text}\" id=\"main\" class=\"foo\">"),
    ),
    (
        "html",
        "input#main.foo|t|c|bem",
        Some("<input type=\"${2:text}\" id=\"main\" class=\"foo\">"),
    ),
    (
        "html",
        "div.block>span.-element+span._modifier|bem",
        Some(
            "<div class=\"block\"><span class=\"block__element\">${1}</span><span class=\"block block_modifier\">${0}</span></div>",
        ),
    ),
    (
        "html",
        "div.block>span.-element+span._modifier|c",
        Some(
            "<div class=\"block\"><span class=\"-element\">${1}</span>\n\t<!-- /.-element --><span class=\"_modifier\">${0}</span>\n\t<!-- /._modifier --></div>\n<!-- /.block -->",
        ),
    ),
    (
        "html",
        "div.block>span.-element+span._modifier|t",
        Some(
            "<div class=\"block\"><span class=\"-element\">${1}</span><span class=\"_modifier\">${0}</span></div>",
        ),
    ),
    (
        "html",
        "div.block>span.-element+span._modifier|bem|c",
        Some(
            "<div class=\"block\"><span class=\"block__element\">${1}</span>\n\t<!-- /.block__element --><span class=\"block block_modifier\">${0}</span>\n\t<!-- /.block block_modifier --></div>\n<!-- /.block -->",
        ),
    ),
    (
        "html",
        "div.block>span.-element+span._modifier|c|bem",
        Some(
            "<div class=\"block\"><span class=\"block__element\">${1}</span>\n\t<!-- /.block__element --><span class=\"block block_modifier\">${0}</span>\n\t<!-- /.block block_modifier --></div>\n<!-- /.block -->",
        ),
    ),
    (
        "html",
        "div.block>span.-element+span._modifier|bem|t|c",
        Some(
            "<div class=\"block\"><span class=\"block__element\">${1}</span>\n\t<!-- /.block__element --><span class=\"block block_modifier\">${0}</span>\n\t<!-- /.block block_modifier --></div>\n<!-- /.block -->",
        ),
    ),
    (
        "html",
        "div.block>span.-element+span._modifier|c|c|c",
        Some(
            "<div class=\"block\"><span class=\"-element\">${1}</span>\n\t<!-- /.-element --><span class=\"_modifier\">${0}</span>\n\t<!-- /._modifier --></div>\n<!-- /.block -->",
        ),
    ),
    (
        "html",
        "div.block>span.-element+span._modifier|bem|bem",
        Some(
            "<div class=\"block\"><span class=\"block__element\">${1}</span><span class=\"block block_modifier\">${0}</span></div>",
        ),
    ),
    (
        "html",
        "div.block>span.-element+span._modifier|c|t",
        Some(
            "<div class=\"block\"><span class=\"-element\">${1}</span>\n\t<!-- /.-element --><span class=\"_modifier\">${0}</span>\n\t<!-- /._modifier --></div>\n<!-- /.block -->",
        ),
    ),
    (
        "html",
        "div.block>span.-element+span._modifier|t|c|bem",
        Some(
            "<div class=\"block\"><span class=\"block__element\">${1}</span>\n\t<!-- /.block__element --><span class=\"block block_modifier\">${0}</span>\n\t<!-- /.block block_modifier --></div>\n<!-- /.block -->",
        ),
    ),
    (
        "html",
        "ul.block>li.-item*3|bem",
        Some(
            "<ul class=\"block\">\n\t<li class=\"block__item\">${1}</li>\n\t<li class=\"block__item\">${2}</li>\n\t<li class=\"block__item\">${0}</li>\n</ul>",
        ),
    ),
    (
        "html",
        "ul.block>li.-item*3|c",
        Some(
            "<ul class=\"block\">\n\t<li class=\"-item\">${1}</li>\n\t<!-- /.-item -->\n\t<li class=\"-item\">${2}</li>\n\t<!-- /.-item -->\n\t<li class=\"-item\">${0}</li>\n\t<!-- /.-item -->\n</ul>\n<!-- /.block -->",
        ),
    ),
    (
        "html",
        "ul.block>li.-item*3|t",
        Some(
            "<ul class=\"block\">\n\t<li class=\"-item\">${1}</li>\n\t<li class=\"-item\">${2}</li>\n\t<li class=\"-item\">${0}</li>\n</ul>",
        ),
    ),
    (
        "html",
        "ul.block>li.-item*3|bem|c",
        Some(
            "<ul class=\"block\">\n\t<li class=\"block__item\">${1}</li>\n\t<!-- /.block__item -->\n\t<li class=\"block__item\">${2}</li>\n\t<!-- /.block__item -->\n\t<li class=\"block__item\">${0}</li>\n\t<!-- /.block__item -->\n</ul>\n<!-- /.block -->",
        ),
    ),
    (
        "html",
        "ul.block>li.-item*3|c|bem",
        Some(
            "<ul class=\"block\">\n\t<li class=\"block__item\">${1}</li>\n\t<!-- /.block__item -->\n\t<li class=\"block__item\">${2}</li>\n\t<!-- /.block__item -->\n\t<li class=\"block__item\">${0}</li>\n\t<!-- /.block__item -->\n</ul>\n<!-- /.block -->",
        ),
    ),
    (
        "html",
        "ul.block>li.-item*3|bem|t|c",
        Some(
            "<ul class=\"block\">\n\t<li class=\"block__item\">${1}</li>\n\t<!-- /.block__item -->\n\t<li class=\"block__item\">${2}</li>\n\t<!-- /.block__item -->\n\t<li class=\"block__item\">${0}</li>\n\t<!-- /.block__item -->\n</ul>\n<!-- /.block -->",
        ),
    ),
    (
        "html",
        "ul.block>li.-item*3|c|c|c",
        Some(
            "<ul class=\"block\">\n\t<li class=\"-item\">${1}</li>\n\t<!-- /.-item -->\n\t<li class=\"-item\">${2}</li>\n\t<!-- /.-item -->\n\t<li class=\"-item\">${0}</li>\n\t<!-- /.-item -->\n</ul>\n<!-- /.block -->",
        ),
    ),
    (
        "html",
        "ul.block>li.-item*3|bem|bem",
        Some(
            "<ul class=\"block\">\n\t<li class=\"block__item\">${1}</li>\n\t<li class=\"block__item\">${2}</li>\n\t<li class=\"block__item\">${0}</li>\n</ul>",
        ),
    ),
    (
        "html",
        "ul.block>li.-item*3|c|t",
        Some(
            "<ul class=\"block\">\n\t<li class=\"-item\">${1}</li>\n\t<!-- /.-item -->\n\t<li class=\"-item\">${2}</li>\n\t<!-- /.-item -->\n\t<li class=\"-item\">${0}</li>\n\t<!-- /.-item -->\n</ul>\n<!-- /.block -->",
        ),
    ),
    (
        "html",
        "ul.block>li.-item*3|t|c|bem",
        Some(
            "<ul class=\"block\">\n\t<li class=\"block__item\">${1}</li>\n\t<!-- /.block__item -->\n\t<li class=\"block__item\">${2}</li>\n\t<!-- /.block__item -->\n\t<li class=\"block__item\">${0}</li>\n\t<!-- /.block__item -->\n</ul>\n<!-- /.block -->",
        ),
    ),
    (
        "html",
        "div..block>.-element_modifier|bem",
        Some(
            "<div class=\"block\">\n\t<div class=\"block__element block__element_modifier\">${0}</div>\n</div>",
        ),
    ),
    (
        "html",
        "div..block>.-element_modifier|c",
        Some(
            "<div class=\"block\">\n\t<div class=\"-element_modifier\">${0}</div>\n\t<!-- /.-element_modifier -->\n</div>\n<!-- /.block -->",
        ),
    ),
    (
        "html",
        "div..block>.-element_modifier|t",
        Some("<div class=\"block\">\n\t<div class=\"-element_modifier\">${0}</div>\n</div>"),
    ),
    (
        "html",
        "div..block>.-element_modifier|bem|c",
        Some(
            "<div class=\"block\">\n\t<div class=\"block__element block__element_modifier\">${0}</div>\n\t<!-- /.block__element block__element_modifier -->\n</div>\n<!-- /.block -->",
        ),
    ),
    (
        "html",
        "div..block>.-element_modifier|c|bem",
        Some(
            "<div class=\"block\">\n\t<div class=\"block__element block__element_modifier\">${0}</div>\n\t<!-- /.block__element block__element_modifier -->\n</div>\n<!-- /.block -->",
        ),
    ),
    (
        "html",
        "div..block>.-element_modifier|bem|t|c",
        Some(
            "<div class=\"block\">\n\t<div class=\"block__element block__element_modifier\">${0}</div>\n\t<!-- /.block__element block__element_modifier -->\n</div>\n<!-- /.block -->",
        ),
    ),
    (
        "html",
        "div..block>.-element_modifier|c|c|c",
        Some(
            "<div class=\"block\">\n\t<div class=\"-element_modifier\">${0}</div>\n\t<!-- /.-element_modifier -->\n</div>\n<!-- /.block -->",
        ),
    ),
    (
        "html",
        "div..block>.-element_modifier|bem|bem",
        Some(
            "<div class=\"block\">\n\t<div class=\"block__element block__element_modifier\">${0}</div>\n</div>",
        ),
    ),
    (
        "html",
        "div..block>.-element_modifier|c|t",
        Some(
            "<div class=\"block\">\n\t<div class=\"-element_modifier\">${0}</div>\n\t<!-- /.-element_modifier -->\n</div>\n<!-- /.block -->",
        ),
    ),
    (
        "html",
        "div..block>.-element_modifier|t|c|bem",
        Some(
            "<div class=\"block\">\n\t<div class=\"block__element block__element_modifier\">${0}</div>\n\t<!-- /.block__element block__element_modifier -->\n</div>\n<!-- /.block -->",
        ),
    ),
    (
        "html",
        "div[class=\"block﻿foo\"]|bem",
        Some("<div class=\"block foo\">${0}</div>"),
    ),
    (
        "html",
        "div[class=\"block﻿foo\"]|c",
        Some("<div class=\"block﻿foo\">${0}</div>\n<!-- /.block﻿foo -->"),
    ),
    (
        "html",
        "div[class=\"block﻿foo\"]|t",
        Some("<div class=\"block﻿foo\">${0}</div>"),
    ),
    (
        "html",
        "div[class=\"block﻿foo\"]|bem|c",
        Some("<div class=\"block foo\">${0}</div>\n<!-- /.block foo -->"),
    ),
    (
        "html",
        "div[class=\"block﻿foo\"]|c|bem",
        Some("<div class=\"block foo\">${0}</div>\n<!-- /.block foo -->"),
    ),
    (
        "html",
        "div[class=\"block﻿foo\"]|bem|t|c",
        Some("<div class=\"block foo\">${0}</div>\n<!-- /.block foo -->"),
    ),
    (
        "html",
        "div[class=\"block﻿foo\"]|c|c|c",
        Some("<div class=\"block﻿foo\">${0}</div>\n<!-- /.block﻿foo -->"),
    ),
    (
        "html",
        "div[class=\"block﻿foo\"]|bem|bem",
        Some("<div class=\"block foo\">${0}</div>"),
    ),
    (
        "html",
        "div[class=\"block﻿foo\"]|c|t",
        Some("<div class=\"block﻿foo\">${0}</div>\n<!-- /.block﻿foo -->"),
    ),
    (
        "html",
        "div[class=\"block﻿foo\"]|t|c|bem",
        Some("<div class=\"block foo\">${0}</div>\n<!-- /.block foo -->"),
    ),
    (
        "html",
        "div[class=\"blockfoo\"]|bem",
        Some("<div class=\"blockfoo\">${0}</div>"),
    ),
    (
        "html",
        "div[class=\"blockfoo\"]|c",
        Some("<div class=\"blockfoo\">${0}</div>\n<!-- /.blockfoo -->"),
    ),
    (
        "html",
        "div[class=\"blockfoo\"]|t",
        Some("<div class=\"blockfoo\">${0}</div>"),
    ),
    (
        "html",
        "div[class=\"blockfoo\"]|bem|c",
        Some("<div class=\"blockfoo\">${0}</div>\n<!-- /.blockfoo -->"),
    ),
    (
        "html",
        "div[class=\"blockfoo\"]|c|bem",
        Some("<div class=\"blockfoo\">${0}</div>\n<!-- /.blockfoo -->"),
    ),
    (
        "html",
        "div[class=\"blockfoo\"]|bem|t|c",
        Some("<div class=\"blockfoo\">${0}</div>\n<!-- /.blockfoo -->"),
    ),
    (
        "html",
        "div[class=\"blockfoo\"]|c|c|c",
        Some("<div class=\"blockfoo\">${0}</div>\n<!-- /.blockfoo -->"),
    ),
    (
        "html",
        "div[class=\"blockfoo\"]|bem|bem",
        Some("<div class=\"blockfoo\">${0}</div>"),
    ),
    (
        "html",
        "div[class=\"blockfoo\"]|c|t",
        Some("<div class=\"blockfoo\">${0}</div>\n<!-- /.blockfoo -->"),
    ),
    (
        "html",
        "div[class=\"blockfoo\"]|t|c|bem",
        Some("<div class=\"blockfoo\">${0}</div>\n<!-- /.blockfoo -->"),
    ),
    (
        "html",
        "div#${1}.block|bem",
        Some("<div id=\"1\" class=\"block\">1</div>"),
    ),
    (
        "html",
        "div#${1}.block|c",
        Some("<div id=\"1\" class=\"block\">1</div>\n<!-- /#1.block -->"),
    ),
    (
        "html",
        "div#${1}.block|t",
        Some("<div id=\"1\" class=\"block\">1</div>"),
    ),
    (
        "html",
        "div#${1}.block|bem|c",
        Some("<div id=\"1\" class=\"block\">1</div>\n<!-- /#1.block -->"),
    ),
    (
        "html",
        "div#${1}.block|c|bem",
        Some("<div id=\"1\" class=\"block\">1</div>\n<!-- /#1.block -->"),
    ),
    (
        "html",
        "div#${1}.block|bem|t|c",
        Some("<div id=\"1\" class=\"block\">1</div>\n<!-- /#1.block -->"),
    ),
    (
        "html",
        "div#${1}.block|c|c|c",
        Some("<div id=\"1\" class=\"block\">1</div>\n<!-- /#1.block -->"),
    ),
    (
        "html",
        "div#${1}.block|bem|bem",
        Some("<div id=\"1\" class=\"block\">1</div>"),
    ),
    (
        "html",
        "div#${1}.block|c|t",
        Some("<div id=\"1\" class=\"block\">1</div>\n<!-- /#1.block -->"),
    ),
    (
        "html",
        "div#${1}.block|t|c|bem",
        Some("<div id=\"1\" class=\"block\">1</div>\n<!-- /#1.block -->"),
    ),
    (
        "html",
        "div#main>span.foo|bem",
        Some("<div id=\"main\"><span class=\"foo\">${0}</span></div>"),
    ),
    (
        "html",
        "div#main>span.foo|c",
        Some(
            "<div id=\"main\"><span class=\"foo\">${0}</span>\n\t<!-- /.foo --></div>\n<!-- /#main -->",
        ),
    ),
    (
        "html",
        "div#main>span.foo|t",
        Some("<div id=\"main\"><span class=\"foo\">${0}</span></div>"),
    ),
    (
        "html",
        "div#main>span.foo|bem|c",
        Some(
            "<div id=\"main\"><span class=\"foo\">${0}</span>\n\t<!-- /.foo --></div>\n<!-- /#main -->",
        ),
    ),
    (
        "html",
        "div#main>span.foo|c|bem",
        Some(
            "<div id=\"main\"><span class=\"foo\">${0}</span>\n\t<!-- /.foo --></div>\n<!-- /#main -->",
        ),
    ),
    (
        "html",
        "div#main>span.foo|bem|t|c",
        Some(
            "<div id=\"main\"><span class=\"foo\">${0}</span>\n\t<!-- /.foo --></div>\n<!-- /#main -->",
        ),
    ),
    (
        "html",
        "div#main>span.foo|c|c|c",
        Some(
            "<div id=\"main\"><span class=\"foo\">${0}</span>\n\t<!-- /.foo --></div>\n<!-- /#main -->",
        ),
    ),
    (
        "html",
        "div#main>span.foo|bem|bem",
        Some("<div id=\"main\"><span class=\"foo\">${0}</span></div>"),
    ),
    (
        "html",
        "div#main>span.foo|c|t",
        Some(
            "<div id=\"main\"><span class=\"foo\">${0}</span>\n\t<!-- /.foo --></div>\n<!-- /#main -->",
        ),
    ),
    (
        "html",
        "div#main>span.foo|t|c|bem",
        Some(
            "<div id=\"main\"><span class=\"foo\">${0}</span>\n\t<!-- /.foo --></div>\n<!-- /#main -->",
        ),
    ),
    (
        "jsx",
        "div.block|bem",
        Some("<div className=\"block\">${0}</div>"),
    ),
    (
        "jsx",
        "div.block|c",
        Some("<div className=\"block\">${0}</div>\n<!-- /.block -->"),
    ),
    (
        "jsx",
        "div.block|t",
        Some("<div className=\"block\">${0}</div>"),
    ),
    (
        "jsx",
        "div.block|bem|c",
        Some("<div className=\"block\">${0}</div>\n<!-- /.block -->"),
    ),
    (
        "jsx",
        "div.block|c|bem",
        Some("<div className=\"block\">${0}</div>\n<!-- /.block -->"),
    ),
    (
        "jsx",
        "div.block|bem|t|c",
        Some("<div className=\"block\">${0}</div>\n<!-- /.block -->"),
    ),
    (
        "jsx",
        "div.block|c|c|c",
        Some("<div className=\"block\">${0}</div>\n<!-- /.block -->"),
    ),
    (
        "jsx",
        "div.block|bem|bem",
        Some("<div className=\"block\">${0}</div>"),
    ),
    (
        "jsx",
        "div.block|c|t",
        Some("<div className=\"block\">${0}</div>\n<!-- /.block -->"),
    ),
    (
        "jsx",
        "div.block|t|c|bem",
        Some("<div className=\"block\">${0}</div>\n<!-- /.block -->"),
    ),
    (
        "jsx",
        "div.block_mod|bem",
        Some("<div className=\"block block_mod\">${0}</div>"),
    ),
    (
        "jsx",
        "div.block_mod|c",
        Some("<div className=\"block_mod\">${0}</div>\n<!-- /.block_mod -->"),
    ),
    (
        "jsx",
        "div.block_mod|t",
        Some("<div className=\"block_mod\">${0}</div>"),
    ),
    (
        "jsx",
        "div.block_mod|bem|c",
        Some("<div className=\"block block_mod\">${0}</div>\n<!-- /.block block_mod -->"),
    ),
    (
        "jsx",
        "div.block_mod|c|bem",
        Some("<div className=\"block block_mod\">${0}</div>\n<!-- /.block block_mod -->"),
    ),
    (
        "jsx",
        "div.block_mod|bem|t|c",
        Some("<div className=\"block block_mod\">${0}</div>\n<!-- /.block block_mod -->"),
    ),
    (
        "jsx",
        "div.block_mod|c|c|c",
        Some("<div className=\"block_mod\">${0}</div>\n<!-- /.block_mod -->"),
    ),
    (
        "jsx",
        "div.block_mod|bem|bem",
        Some("<div className=\"block block_mod\">${0}</div>"),
    ),
    (
        "jsx",
        "div.block_mod|c|t",
        Some("<div className=\"block_mod\">${0}</div>\n<!-- /.block_mod -->"),
    ),
    (
        "jsx",
        "div.block_mod|t|c|bem",
        Some("<div className=\"block block_mod\">${0}</div>\n<!-- /.block block_mod -->"),
    ),
    (
        "jsx",
        "div.block__element_mod|bem",
        Some("<div className=\"block block_element_mod\">${0}</div>"),
    ),
    (
        "jsx",
        "div.block__element_mod|c",
        Some("<div className=\"block__element_mod\">${0}</div>\n<!-- /.block__element_mod -->"),
    ),
    (
        "jsx",
        "div.block__element_mod|t",
        Some("<div className=\"block__element_mod\">${0}</div>"),
    ),
    (
        "jsx",
        "div.block__element_mod|bem|c",
        Some(
            "<div className=\"block block_element_mod\">${0}</div>\n<!-- /.block block_element_mod -->",
        ),
    ),
    (
        "jsx",
        "div.block__element_mod|c|bem",
        Some(
            "<div className=\"block block_element_mod\">${0}</div>\n<!-- /.block block_element_mod -->",
        ),
    ),
    (
        "jsx",
        "div.block__element_mod|bem|t|c",
        Some(
            "<div className=\"block block_element_mod\">${0}</div>\n<!-- /.block block_element_mod -->",
        ),
    ),
    (
        "jsx",
        "div.block__element_mod|c|c|c",
        Some("<div className=\"block__element_mod\">${0}</div>\n<!-- /.block__element_mod -->"),
    ),
    (
        "jsx",
        "div.block__element_mod|bem|bem",
        Some("<div className=\"block block_element_mod\">${0}</div>"),
    ),
    (
        "jsx",
        "div.block__element_mod|c|t",
        Some("<div className=\"block__element_mod\">${0}</div>\n<!-- /.block__element_mod -->"),
    ),
    (
        "jsx",
        "div.block__element_mod|t|c|bem",
        Some(
            "<div className=\"block block_element_mod\">${0}</div>\n<!-- /.block block_element_mod -->",
        ),
    ),
    (
        "jsx",
        "div.b-block.other|bem",
        Some("<div className=\"b-block other\">${0}</div>"),
    ),
    (
        "jsx",
        "div.b-block.other|c",
        Some("<div className=\"b-block other\">${0}</div>\n<!-- /.b-block other -->"),
    ),
    (
        "jsx",
        "div.b-block.other|t",
        Some("<div className=\"b-block other\">${0}</div>"),
    ),
    (
        "jsx",
        "div.b-block.other|bem|c",
        Some("<div className=\"b-block other\">${0}</div>\n<!-- /.b-block other -->"),
    ),
    (
        "jsx",
        "div.b-block.other|c|bem",
        Some("<div className=\"b-block other\">${0}</div>\n<!-- /.b-block other -->"),
    ),
    (
        "jsx",
        "div.b-block.other|bem|t|c",
        Some("<div className=\"b-block other\">${0}</div>\n<!-- /.b-block other -->"),
    ),
    (
        "jsx",
        "div.b-block.other|c|c|c",
        Some("<div className=\"b-block other\">${0}</div>\n<!-- /.b-block other -->"),
    ),
    (
        "jsx",
        "div.b-block.other|bem|bem",
        Some("<div className=\"b-block other\">${0}</div>"),
    ),
    (
        "jsx",
        "div.b-block.other|c|t",
        Some("<div className=\"b-block other\">${0}</div>\n<!-- /.b-block other -->"),
    ),
    (
        "jsx",
        "div.b-block.other|t|c|bem",
        Some("<div className=\"b-block other\">${0}</div>\n<!-- /.b-block other -->"),
    ),
    (
        "jsx",
        "div.-element|bem",
        Some("<div className=\"__element\">${0}</div>"),
    ),
    (
        "jsx",
        "div.-element|c",
        Some("<div className=\"-element\">${0}</div>\n<!-- /.-element -->"),
    ),
    (
        "jsx",
        "div.-element|t",
        Some("<div className=\"-element\">${0}</div>"),
    ),
    (
        "jsx",
        "div.-element|bem|c",
        Some("<div className=\"__element\">${0}</div>\n<!-- /.__element -->"),
    ),
    (
        "jsx",
        "div.-element|c|bem",
        Some("<div className=\"__element\">${0}</div>\n<!-- /.__element -->"),
    ),
    (
        "jsx",
        "div.-element|bem|t|c",
        Some("<div className=\"__element\">${0}</div>\n<!-- /.__element -->"),
    ),
    (
        "jsx",
        "div.-element|c|c|c",
        Some("<div className=\"-element\">${0}</div>\n<!-- /.-element -->"),
    ),
    (
        "jsx",
        "div.-element|bem|bem",
        Some("<div className=\"__element\">${0}</div>"),
    ),
    (
        "jsx",
        "div.-element|c|t",
        Some("<div className=\"-element\">${0}</div>\n<!-- /.-element -->"),
    ),
    (
        "jsx",
        "div.-element|t|c|bem",
        Some("<div className=\"__element\">${0}</div>\n<!-- /.__element -->"),
    ),
    (
        "jsx",
        "div._modifier|bem",
        Some("<div className=\"_modifier\">${0}</div>"),
    ),
    (
        "jsx",
        "div._modifier|c",
        Some("<div className=\"_modifier\">${0}</div>\n<!-- /._modifier -->"),
    ),
    (
        "jsx",
        "div._modifier|t",
        Some("<div className=\"_modifier\">${0}</div>"),
    ),
    (
        "jsx",
        "div._modifier|bem|c",
        Some("<div className=\"_modifier\">${0}</div>\n<!-- /._modifier -->"),
    ),
    (
        "jsx",
        "div._modifier|c|bem",
        Some("<div className=\"_modifier\">${0}</div>\n<!-- /._modifier -->"),
    ),
    (
        "jsx",
        "div._modifier|bem|t|c",
        Some("<div className=\"_modifier\">${0}</div>\n<!-- /._modifier -->"),
    ),
    (
        "jsx",
        "div._modifier|c|c|c",
        Some("<div className=\"_modifier\">${0}</div>\n<!-- /._modifier -->"),
    ),
    (
        "jsx",
        "div._modifier|bem|bem",
        Some("<div className=\"_modifier\">${0}</div>"),
    ),
    (
        "jsx",
        "div._modifier|c|t",
        Some("<div className=\"_modifier\">${0}</div>\n<!-- /._modifier -->"),
    ),
    (
        "jsx",
        "div._modifier|t|c|bem",
        Some("<div className=\"_modifier\">${0}</div>\n<!-- /._modifier -->"),
    ),
    (
        "jsx",
        "div.block>.-element|bem",
        Some("<div className=\"block\">\n\t<div className=\"block__element\">${0}</div>\n</div>"),
    ),
    (
        "jsx",
        "div.block>.-element|c",
        Some(
            "<div className=\"block\">\n\t<div className=\"-element\">${0}</div>\n\t<!-- /.-element -->\n</div>\n<!-- /.block -->",
        ),
    ),
    (
        "jsx",
        "div.block>.-element|t",
        Some("<div className=\"block\">\n\t<div className=\"-element\">${0}</div>\n</div>"),
    ),
    (
        "jsx",
        "div.block>.-element|bem|c",
        Some(
            "<div className=\"block\">\n\t<div className=\"block__element\">${0}</div>\n\t<!-- /.block__element -->\n</div>\n<!-- /.block -->",
        ),
    ),
    (
        "jsx",
        "div.block>.-element|c|bem",
        Some(
            "<div className=\"block\">\n\t<div className=\"block__element\">${0}</div>\n\t<!-- /.block__element -->\n</div>\n<!-- /.block -->",
        ),
    ),
    (
        "jsx",
        "div.block>.-element|bem|t|c",
        Some(
            "<div className=\"block\">\n\t<div className=\"block__element\">${0}</div>\n\t<!-- /.block__element -->\n</div>\n<!-- /.block -->",
        ),
    ),
    (
        "jsx",
        "div.block>.-element|c|c|c",
        Some(
            "<div className=\"block\">\n\t<div className=\"-element\">${0}</div>\n\t<!-- /.-element -->\n</div>\n<!-- /.block -->",
        ),
    ),
    (
        "jsx",
        "div.block>.-element|bem|bem",
        Some("<div className=\"block\">\n\t<div className=\"block__element\">${0}</div>\n</div>"),
    ),
    (
        "jsx",
        "div.block>.-element|c|t",
        Some(
            "<div className=\"block\">\n\t<div className=\"-element\">${0}</div>\n\t<!-- /.-element -->\n</div>\n<!-- /.block -->",
        ),
    ),
    (
        "jsx",
        "div.block>.-element|t|c|bem",
        Some(
            "<div className=\"block\">\n\t<div className=\"block__element\">${0}</div>\n\t<!-- /.block__element -->\n</div>\n<!-- /.block -->",
        ),
    ),
    (
        "jsx",
        "div.block>.-element_modifier|bem",
        Some(
            "<div className=\"block\">\n\t<div className=\"block__element block__element_modifier\">${0}</div>\n</div>",
        ),
    ),
    (
        "jsx",
        "div.block>.-element_modifier|c",
        Some(
            "<div className=\"block\">\n\t<div className=\"-element_modifier\">${0}</div>\n\t<!-- /.-element_modifier -->\n</div>\n<!-- /.block -->",
        ),
    ),
    (
        "jsx",
        "div.block>.-element_modifier|t",
        Some(
            "<div className=\"block\">\n\t<div className=\"-element_modifier\">${0}</div>\n</div>",
        ),
    ),
    (
        "jsx",
        "div.block>.-element_modifier|bem|c",
        Some(
            "<div className=\"block\">\n\t<div className=\"block__element block__element_modifier\">${0}</div>\n\t<!-- /.block__element block__element_modifier -->\n</div>\n<!-- /.block -->",
        ),
    ),
    (
        "jsx",
        "div.block>.-element_modifier|c|bem",
        Some(
            "<div className=\"block\">\n\t<div className=\"block__element block__element_modifier\">${0}</div>\n\t<!-- /.block__element block__element_modifier -->\n</div>\n<!-- /.block -->",
        ),
    ),
    (
        "jsx",
        "div.block>.-element_modifier|bem|t|c",
        Some(
            "<div className=\"block\">\n\t<div className=\"block__element block__element_modifier\">${0}</div>\n\t<!-- /.block__element block__element_modifier -->\n</div>\n<!-- /.block -->",
        ),
    ),
    (
        "jsx",
        "div.block>.-element_modifier|c|c|c",
        Some(
            "<div className=\"block\">\n\t<div className=\"-element_modifier\">${0}</div>\n\t<!-- /.-element_modifier -->\n</div>\n<!-- /.block -->",
        ),
    ),
    (
        "jsx",
        "div.block>.-element_modifier|bem|bem",
        Some(
            "<div className=\"block\">\n\t<div className=\"block__element block__element_modifier\">${0}</div>\n</div>",
        ),
    ),
    (
        "jsx",
        "div.block>.-element_modifier|c|t",
        Some(
            "<div className=\"block\">\n\t<div className=\"-element_modifier\">${0}</div>\n\t<!-- /.-element_modifier -->\n</div>\n<!-- /.block -->",
        ),
    ),
    (
        "jsx",
        "div.block>.-element_modifier|t|c|bem",
        Some(
            "<div className=\"block\">\n\t<div className=\"block__element block__element_modifier\">${0}</div>\n\t<!-- /.block__element block__element_modifier -->\n</div>\n<!-- /.block -->",
        ),
    ),
    (
        "jsx",
        "div.block>.-element>._modifier|bem",
        Some(
            "<div className=\"block\">\n\t<div className=\"block__element\">\n\t\t<div className=\"block block_modifier\">${0}</div>\n\t</div>\n</div>",
        ),
    ),
    (
        "jsx",
        "div.block>.-element>._modifier|c",
        Some(
            "<div className=\"block\">\n\t<div className=\"-element\">\n\t\t<div className=\"_modifier\">${0}</div>\n\t\t<!-- /._modifier -->\n\t</div>\n\t<!-- /.-element -->\n</div>\n<!-- /.block -->",
        ),
    ),
    (
        "jsx",
        "div.block>.-element>._modifier|t",
        Some(
            "<div className=\"block\">\n\t<div className=\"-element\">\n\t\t<div className=\"_modifier\">${0}</div>\n\t</div>\n</div>",
        ),
    ),
    (
        "jsx",
        "div.block>.-element>._modifier|bem|c",
        Some(
            "<div className=\"block\">\n\t<div className=\"block__element\">\n\t\t<div className=\"block block_modifier\">${0}</div>\n\t\t<!-- /.block block_modifier -->\n\t</div>\n\t<!-- /.block__element -->\n</div>\n<!-- /.block -->",
        ),
    ),
    (
        "jsx",
        "div.block>.-element>._modifier|c|bem",
        Some(
            "<div className=\"block\">\n\t<div className=\"block__element\">\n\t\t<div className=\"block block_modifier\">${0}</div>\n\t\t<!-- /.block block_modifier -->\n\t</div>\n\t<!-- /.block__element -->\n</div>\n<!-- /.block -->",
        ),
    ),
    (
        "jsx",
        "div.block>.-element>._modifier|bem|t|c",
        Some(
            "<div className=\"block\">\n\t<div className=\"block__element\">\n\t\t<div className=\"block block_modifier\">${0}</div>\n\t\t<!-- /.block block_modifier -->\n\t</div>\n\t<!-- /.block__element -->\n</div>\n<!-- /.block -->",
        ),
    ),
    (
        "jsx",
        "div.block>.-element>._modifier|c|c|c",
        Some(
            "<div className=\"block\">\n\t<div className=\"-element\">\n\t\t<div className=\"_modifier\">${0}</div>\n\t\t<!-- /._modifier -->\n\t</div>\n\t<!-- /.-element -->\n</div>\n<!-- /.block -->",
        ),
    ),
    (
        "jsx",
        "div.block>.-element>._modifier|bem|bem",
        Some(
            "<div className=\"block\">\n\t<div className=\"block__element\">\n\t\t<div className=\"block block_modifier\">${0}</div>\n\t</div>\n</div>",
        ),
    ),
    (
        "jsx",
        "div.block>.-element>._modifier|c|t",
        Some(
            "<div className=\"block\">\n\t<div className=\"-element\">\n\t\t<div className=\"_modifier\">${0}</div>\n\t\t<!-- /._modifier -->\n\t</div>\n\t<!-- /.-element -->\n</div>\n<!-- /.block -->",
        ),
    ),
    (
        "jsx",
        "div.block>.-element>._modifier|t|c|bem",
        Some(
            "<div className=\"block\">\n\t<div className=\"block__element\">\n\t\t<div className=\"block block_modifier\">${0}</div>\n\t\t<!-- /.block block_modifier -->\n\t</div>\n\t<!-- /.block__element -->\n</div>\n<!-- /.block -->",
        ),
    ),
    (
        "jsx",
        "div.block>.-element>.__modifier|bem",
        Some(
            "<div className=\"block\">\n\t<div className=\"block__element\">\n\t\t<div className=\"block block_modifier\">${0}</div>\n\t</div>\n</div>",
        ),
    ),
    (
        "jsx",
        "div.block>.-element>.__modifier|c",
        Some(
            "<div className=\"block\">\n\t<div className=\"-element\">\n\t\t<div className=\"__modifier\">${0}</div>\n\t\t<!-- /.__modifier -->\n\t</div>\n\t<!-- /.-element -->\n</div>\n<!-- /.block -->",
        ),
    ),
    (
        "jsx",
        "div.block>.-element>.__modifier|t",
        Some(
            "<div className=\"block\">\n\t<div className=\"-element\">\n\t\t<div className=\"__modifier\">${0}</div>\n\t</div>\n</div>",
        ),
    ),
    (
        "jsx",
        "div.block>.-element>.__modifier|bem|c",
        Some(
            "<div className=\"block\">\n\t<div className=\"block__element\">\n\t\t<div className=\"block block_modifier\">${0}</div>\n\t\t<!-- /.block block_modifier -->\n\t</div>\n\t<!-- /.block__element -->\n</div>\n<!-- /.block -->",
        ),
    ),
    (
        "jsx",
        "div.block>.-element>.__modifier|c|bem",
        Some(
            "<div className=\"block\">\n\t<div className=\"block__element\">\n\t\t<div className=\"block block_modifier\">${0}</div>\n\t\t<!-- /.block block_modifier -->\n\t</div>\n\t<!-- /.block__element -->\n</div>\n<!-- /.block -->",
        ),
    ),
    (
        "jsx",
        "div.block>.-element>.__modifier|bem|t|c",
        Some(
            "<div className=\"block\">\n\t<div className=\"block__element\">\n\t\t<div className=\"block block_modifier\">${0}</div>\n\t\t<!-- /.block block_modifier -->\n\t</div>\n\t<!-- /.block__element -->\n</div>\n<!-- /.block -->",
        ),
    ),
    (
        "jsx",
        "div.block>.-element>.__modifier|c|c|c",
        Some(
            "<div className=\"block\">\n\t<div className=\"-element\">\n\t\t<div className=\"__modifier\">${0}</div>\n\t\t<!-- /.__modifier -->\n\t</div>\n\t<!-- /.-element -->\n</div>\n<!-- /.block -->",
        ),
    ),
    (
        "jsx",
        "div.block>.-element>.__modifier|bem|bem",
        Some(
            "<div className=\"block\">\n\t<div className=\"block__element\">\n\t\t<div className=\"block block_modifier\">${0}</div>\n\t</div>\n</div>",
        ),
    ),
    (
        "jsx",
        "div.block>.-element>.__modifier|c|t",
        Some(
            "<div className=\"block\">\n\t<div className=\"-element\">\n\t\t<div className=\"__modifier\">${0}</div>\n\t\t<!-- /.__modifier -->\n\t</div>\n\t<!-- /.-element -->\n</div>\n<!-- /.block -->",
        ),
    ),
    (
        "jsx",
        "div.block>.-element>.__modifier|t|c|bem",
        Some(
            "<div className=\"block\">\n\t<div className=\"block__element\">\n\t\t<div className=\"block block_modifier\">${0}</div>\n\t\t<!-- /.block block_modifier -->\n\t</div>\n\t<!-- /.block__element -->\n</div>\n<!-- /.block -->",
        ),
    ),
    (
        "jsx",
        "div.outer>div.inner>.--element_modifier|bem",
        Some(
            "<div className=\"outer\">\n\t<div className=\"inner\">\n\t\t<div className=\"inner__element inner__element_modifier\">${0}</div>\n\t</div>\n</div>",
        ),
    ),
    (
        "jsx",
        "div.outer>div.inner>.--element_modifier|c",
        Some(
            "<div className=\"outer\">\n\t<div className=\"inner\">\n\t\t<div className=\"--element_modifier\">${0}</div>\n\t\t<!-- /.--element_modifier -->\n\t</div>\n\t<!-- /.inner -->\n</div>\n<!-- /.outer -->",
        ),
    ),
    (
        "jsx",
        "div.outer>div.inner>.--element_modifier|t",
        Some(
            "<div className=\"outer\">\n\t<div className=\"inner\">\n\t\t<div className=\"--element_modifier\">${0}</div>\n\t</div>\n</div>",
        ),
    ),
    (
        "jsx",
        "div.outer>div.inner>.--element_modifier|bem|c",
        Some(
            "<div className=\"outer\">\n\t<div className=\"inner\">\n\t\t<div className=\"inner__element inner__element_modifier\">${0}</div>\n\t\t<!-- /.inner__element inner__element_modifier -->\n\t</div>\n\t<!-- /.inner -->\n</div>\n<!-- /.outer -->",
        ),
    ),
    (
        "jsx",
        "div.outer>div.inner>.--element_modifier|c|bem",
        Some(
            "<div className=\"outer\">\n\t<div className=\"inner\">\n\t\t<div className=\"inner__element inner__element_modifier\">${0}</div>\n\t\t<!-- /.inner__element inner__element_modifier -->\n\t</div>\n\t<!-- /.inner -->\n</div>\n<!-- /.outer -->",
        ),
    ),
    (
        "jsx",
        "div.outer>div.inner>.--element_modifier|bem|t|c",
        Some(
            "<div className=\"outer\">\n\t<div className=\"inner\">\n\t\t<div className=\"inner__element inner__element_modifier\">${0}</div>\n\t\t<!-- /.inner__element inner__element_modifier -->\n\t</div>\n\t<!-- /.inner -->\n</div>\n<!-- /.outer -->",
        ),
    ),
    (
        "jsx",
        "div.outer>div.inner>.--element_modifier|c|c|c",
        Some(
            "<div className=\"outer\">\n\t<div className=\"inner\">\n\t\t<div className=\"--element_modifier\">${0}</div>\n\t\t<!-- /.--element_modifier -->\n\t</div>\n\t<!-- /.inner -->\n</div>\n<!-- /.outer -->",
        ),
    ),
    (
        "jsx",
        "div.outer>div.inner>.--element_modifier|bem|bem",
        Some(
            "<div className=\"outer\">\n\t<div className=\"inner\">\n\t\t<div className=\"inner__element inner__element_modifier\">${0}</div>\n\t</div>\n</div>",
        ),
    ),
    (
        "jsx",
        "div.outer>div.inner>.--element_modifier|c|t",
        Some(
            "<div className=\"outer\">\n\t<div className=\"inner\">\n\t\t<div className=\"--element_modifier\">${0}</div>\n\t\t<!-- /.--element_modifier -->\n\t</div>\n\t<!-- /.inner -->\n</div>\n<!-- /.outer -->",
        ),
    ),
    (
        "jsx",
        "div.outer>div.inner>.--element_modifier|t|c|bem",
        Some(
            "<div className=\"outer\">\n\t<div className=\"inner\">\n\t\t<div className=\"inner__element inner__element_modifier\">${0}</div>\n\t\t<!-- /.inner__element inner__element_modifier -->\n\t</div>\n\t<!-- /.inner -->\n</div>\n<!-- /.outer -->",
        ),
    ),
    (
        "jsx",
        "div.outer>div.inner>.__modifier|bem",
        Some(
            "<div className=\"outer\">\n\t<div className=\"inner\">\n\t\t<div className=\"inner inner_modifier\">${0}</div>\n\t</div>\n</div>",
        ),
    ),
    (
        "jsx",
        "div.outer>div.inner>.__modifier|c",
        Some(
            "<div className=\"outer\">\n\t<div className=\"inner\">\n\t\t<div className=\"__modifier\">${0}</div>\n\t\t<!-- /.__modifier -->\n\t</div>\n\t<!-- /.inner -->\n</div>\n<!-- /.outer -->",
        ),
    ),
    (
        "jsx",
        "div.outer>div.inner>.__modifier|t",
        Some(
            "<div className=\"outer\">\n\t<div className=\"inner\">\n\t\t<div className=\"__modifier\">${0}</div>\n\t</div>\n</div>",
        ),
    ),
    (
        "jsx",
        "div.outer>div.inner>.__modifier|bem|c",
        Some(
            "<div className=\"outer\">\n\t<div className=\"inner\">\n\t\t<div className=\"inner inner_modifier\">${0}</div>\n\t\t<!-- /.inner inner_modifier -->\n\t</div>\n\t<!-- /.inner -->\n</div>\n<!-- /.outer -->",
        ),
    ),
    (
        "jsx",
        "div.outer>div.inner>.__modifier|c|bem",
        Some(
            "<div className=\"outer\">\n\t<div className=\"inner\">\n\t\t<div className=\"inner inner_modifier\">${0}</div>\n\t\t<!-- /.inner inner_modifier -->\n\t</div>\n\t<!-- /.inner -->\n</div>\n<!-- /.outer -->",
        ),
    ),
    (
        "jsx",
        "div.outer>div.inner>.__modifier|bem|t|c",
        Some(
            "<div className=\"outer\">\n\t<div className=\"inner\">\n\t\t<div className=\"inner inner_modifier\">${0}</div>\n\t\t<!-- /.inner inner_modifier -->\n\t</div>\n\t<!-- /.inner -->\n</div>\n<!-- /.outer -->",
        ),
    ),
    (
        "jsx",
        "div.outer>div.inner>.__modifier|c|c|c",
        Some(
            "<div className=\"outer\">\n\t<div className=\"inner\">\n\t\t<div className=\"__modifier\">${0}</div>\n\t\t<!-- /.__modifier -->\n\t</div>\n\t<!-- /.inner -->\n</div>\n<!-- /.outer -->",
        ),
    ),
    (
        "jsx",
        "div.outer>div.inner>.__modifier|bem|bem",
        Some(
            "<div className=\"outer\">\n\t<div className=\"inner\">\n\t\t<div className=\"inner inner_modifier\">${0}</div>\n\t</div>\n</div>",
        ),
    ),
    (
        "jsx",
        "div.outer>div.inner>.__modifier|c|t",
        Some(
            "<div className=\"outer\">\n\t<div className=\"inner\">\n\t\t<div className=\"__modifier\">${0}</div>\n\t\t<!-- /.__modifier -->\n\t</div>\n\t<!-- /.inner -->\n</div>\n<!-- /.outer -->",
        ),
    ),
    (
        "jsx",
        "div.outer>div.inner>.__modifier|t|c|bem",
        Some(
            "<div className=\"outer\">\n\t<div className=\"inner\">\n\t\t<div className=\"inner inner_modifier\">${0}</div>\n\t\t<!-- /.inner inner_modifier -->\n\t</div>\n\t<!-- /.inner -->\n</div>\n<!-- /.outer -->",
        ),
    ),
    (
        "jsx",
        "div.block.-element_modifier|bem",
        Some("<div className=\"block block__element block__element_modifier\">${0}</div>"),
    ),
    (
        "jsx",
        "div.block.-element_modifier|c",
        Some(
            "<div className=\"block -element_modifier\">${0}</div>\n<!-- /.block -element_modifier -->",
        ),
    ),
    (
        "jsx",
        "div.block.-element_modifier|t",
        Some("<div className=\"block -element_modifier\">${0}</div>"),
    ),
    (
        "jsx",
        "div.block.-element_modifier|bem|c",
        Some(
            "<div className=\"block block__element block__element_modifier\">${0}</div>\n<!-- /.block block__element block__element_modifier -->",
        ),
    ),
    (
        "jsx",
        "div.block.-element_modifier|c|bem",
        Some(
            "<div className=\"block block__element block__element_modifier\">${0}</div>\n<!-- /.block block__element block__element_modifier -->",
        ),
    ),
    (
        "jsx",
        "div.block.-element_modifier|bem|t|c",
        Some(
            "<div className=\"block block__element block__element_modifier\">${0}</div>\n<!-- /.block block__element block__element_modifier -->",
        ),
    ),
    (
        "jsx",
        "div.block.-element_modifier|c|c|c",
        Some(
            "<div className=\"block -element_modifier\">${0}</div>\n<!-- /.block -element_modifier -->",
        ),
    ),
    (
        "jsx",
        "div.block.-element_modifier|bem|bem",
        Some("<div className=\"block block__element block__element_modifier\">${0}</div>"),
    ),
    (
        "jsx",
        "div.block.-element_modifier|c|t",
        Some(
            "<div className=\"block -element_modifier\">${0}</div>\n<!-- /.block -element_modifier -->",
        ),
    ),
    (
        "jsx",
        "div.block.-element_modifier|t|c|bem",
        Some(
            "<div className=\"block block__element block__element_modifier\">${0}</div>\n<!-- /.block block__element block__element_modifier -->",
        ),
    ),
    (
        "jsx",
        "div.block.foo_bar|bem",
        Some("<div className=\"block foo block_bar\">${0}</div>"),
    ),
    (
        "jsx",
        "div.block.foo_bar|c",
        Some("<div className=\"block foo_bar\">${0}</div>\n<!-- /.block foo_bar -->"),
    ),
    (
        "jsx",
        "div.block.foo_bar|t",
        Some("<div className=\"block foo_bar\">${0}</div>"),
    ),
    (
        "jsx",
        "div.block.foo_bar|bem|c",
        Some("<div className=\"block foo block_bar\">${0}</div>\n<!-- /.block foo block_bar -->"),
    ),
    (
        "jsx",
        "div.block.foo_bar|c|bem",
        Some("<div className=\"block foo block_bar\">${0}</div>\n<!-- /.block foo block_bar -->"),
    ),
    (
        "jsx",
        "div.block.foo_bar|bem|t|c",
        Some("<div className=\"block foo block_bar\">${0}</div>\n<!-- /.block foo block_bar -->"),
    ),
    (
        "jsx",
        "div.block.foo_bar|c|c|c",
        Some("<div className=\"block foo_bar\">${0}</div>\n<!-- /.block foo_bar -->"),
    ),
    (
        "jsx",
        "div.block.foo_bar|bem|bem",
        Some("<div className=\"block foo block_bar\">${0}</div>"),
    ),
    (
        "jsx",
        "div.block.foo_bar|c|t",
        Some("<div className=\"block foo_bar\">${0}</div>\n<!-- /.block foo_bar -->"),
    ),
    (
        "jsx",
        "div.block.foo_bar|t|c|bem",
        Some("<div className=\"block foo block_bar\">${0}</div>\n<!-- /.block foo block_bar -->"),
    ),
    (
        "jsx",
        "div.block[class=other]|bem",
        Some("<div className=\"block other\">${0}</div>"),
    ),
    (
        "jsx",
        "div.block[class=other]|c",
        Some("<div className=\"block other\">${0}</div>\n<!-- /.block other -->"),
    ),
    (
        "jsx",
        "div.block[class=other]|t",
        Some("<div className=\"block other\">${0}</div>"),
    ),
    (
        "jsx",
        "div.block[class=other]|bem|c",
        Some("<div className=\"block other\">${0}</div>\n<!-- /.block other -->"),
    ),
    (
        "jsx",
        "div.block[class=other]|c|bem",
        Some("<div className=\"block other\">${0}</div>\n<!-- /.block other -->"),
    ),
    (
        "jsx",
        "div.block[class=other]|bem|t|c",
        Some("<div className=\"block other\">${0}</div>\n<!-- /.block other -->"),
    ),
    (
        "jsx",
        "div.block[class=other]|c|c|c",
        Some("<div className=\"block other\">${0}</div>\n<!-- /.block other -->"),
    ),
    (
        "jsx",
        "div.block[class=other]|bem|bem",
        Some("<div className=\"block other\">${0}</div>"),
    ),
    (
        "jsx",
        "div.block[class=other]|c|t",
        Some("<div className=\"block other\">${0}</div>\n<!-- /.block other -->"),
    ),
    (
        "jsx",
        "div.block[class=other]|t|c|bem",
        Some("<div className=\"block other\">${0}</div>\n<!-- /.block other -->"),
    ),
    (
        "jsx",
        "div.block[class=\"foo bar\"]|bem",
        Some("<div className=\"block foo bar\">${0}</div>"),
    ),
    (
        "jsx",
        "div.block[class=\"foo bar\"]|c",
        Some("<div className=\"block foo bar\">${0}</div>\n<!-- /.block foo bar -->"),
    ),
    (
        "jsx",
        "div.block[class=\"foo bar\"]|t",
        Some("<div className=\"block foo bar\">${0}</div>"),
    ),
    (
        "jsx",
        "div.block[class=\"foo bar\"]|bem|c",
        Some("<div className=\"block foo bar\">${0}</div>\n<!-- /.block foo bar -->"),
    ),
    (
        "jsx",
        "div.block[class=\"foo bar\"]|c|bem",
        Some("<div className=\"block foo bar\">${0}</div>\n<!-- /.block foo bar -->"),
    ),
    (
        "jsx",
        "div.block[class=\"foo bar\"]|bem|t|c",
        Some("<div className=\"block foo bar\">${0}</div>\n<!-- /.block foo bar -->"),
    ),
    (
        "jsx",
        "div.block[class=\"foo bar\"]|c|c|c",
        Some("<div className=\"block foo bar\">${0}</div>\n<!-- /.block foo bar -->"),
    ),
    (
        "jsx",
        "div.block[class=\"foo bar\"]|bem|bem",
        Some("<div className=\"block foo bar\">${0}</div>"),
    ),
    (
        "jsx",
        "div.block[class=\"foo bar\"]|c|t",
        Some("<div className=\"block foo bar\">${0}</div>\n<!-- /.block foo bar -->"),
    ),
    (
        "jsx",
        "div.block[class=\"foo bar\"]|t|c|bem",
        Some("<div className=\"block foo bar\">${0}</div>\n<!-- /.block foo bar -->"),
    ),
    (
        "jsx",
        "div[class=\"${1:foo}\"]|bem",
        Some("<div className=\"foo\">${0}</div>"),
    ),
    (
        "jsx",
        "div[class=\"${1:foo}\"]|c",
        Some("<div className=\"${2:foo}\">${3}</div>\n<!-- /.${5:foo} -->"),
    ),
    (
        "jsx",
        "div[class=\"${1:foo}\"]|t",
        Some("<div className=\"${2:foo}\">${0}</div>"),
    ),
    (
        "jsx",
        "div[class=\"${1:foo}\"]|bem|c",
        Some("<div className=\"foo\">${0}</div>\n<!-- /.foo -->"),
    ),
    (
        "jsx",
        "div[class=\"${1:foo}\"]|c|bem",
        Some("<div className=\"foo\">${0}</div>\n<!-- /.foo -->"),
    ),
    (
        "jsx",
        "div[class=\"${1:foo}\"]|bem|t|c",
        Some("<div className=\"foo\">${0}</div>\n<!-- /.foo -->"),
    ),
    (
        "jsx",
        "div[class=\"${1:foo}\"]|c|c|c",
        Some("<div className=\"${2:foo}\">${3}</div>\n<!-- /.${5:foo} -->"),
    ),
    (
        "jsx",
        "div[class=\"${1:foo}\"]|bem|bem",
        Some("<div className=\"foo\">${0}</div>"),
    ),
    (
        "jsx",
        "div[class=\"${1:foo}\"]|c|t",
        Some("<div className=\"${2:foo}\">${3}</div>\n<!-- /.${5:foo} -->"),
    ),
    (
        "jsx",
        "div[class=\"${1:foo}\"]|t|c|bem",
        Some("<div className=\"foo\">${0}</div>\n<!-- /.foo -->"),
    ),
    (
        "jsx",
        "div[class=\"${1}\"]|bem",
        Some("<div className=\"${2}\">${0}</div>"),
    ),
    (
        "jsx",
        "div[class=\"${1}\"]|c",
        Some("<div className=\"${2}\">${3}</div>\n<!-- /.${0} -->"),
    ),
    (
        "jsx",
        "div[class=\"${1}\"]|t",
        Some("<div className=\"${2}\">${0}</div>"),
    ),
    (
        "jsx",
        "div[class=\"${1}\"]|bem|c",
        Some("<div className=\"${2}\">${3}</div>\n<!-- /.${0} -->"),
    ),
    (
        "jsx",
        "div[class=\"${1}\"]|c|bem",
        Some("<div className=\"${2}\">${3}</div>\n<!-- /.${0} -->"),
    ),
    (
        "jsx",
        "div[class=\"${1}\"]|bem|t|c",
        Some("<div className=\"${2}\">${3}</div>\n<!-- /.${0} -->"),
    ),
    (
        "jsx",
        "div[class=\"${1}\"]|c|c|c",
        Some("<div className=\"${2}\">${3}</div>\n<!-- /.${0} -->"),
    ),
    (
        "jsx",
        "div[class=\"${1}\"]|bem|bem",
        Some("<div className=\"${2}\">${0}</div>"),
    ),
    (
        "jsx",
        "div[class=\"${1}\"]|c|t",
        Some("<div className=\"${2}\">${3}</div>\n<!-- /.${0} -->"),
    ),
    (
        "jsx",
        "div[class=\"${1}\"]|t|c|bem",
        Some("<div className=\"${2}\">${3}</div>\n<!-- /.${0} -->"),
    ),
    (
        "jsx",
        "div.block[!class]|bem",
        Some("<div className=\"block\">${0}</div>"),
    ),
    (
        "jsx",
        "div.block[!class]|c",
        Some("<div className=\"block\">${0}</div>\n<!-- /.block -->"),
    ),
    (
        "jsx",
        "div.block[!class]|t",
        Some("<div className=\"block\">${0}</div>"),
    ),
    (
        "jsx",
        "div.block[!class]|bem|c",
        Some("<div className=\"block\">${0}</div>\n<!-- /.block -->"),
    ),
    (
        "jsx",
        "div.block[!class]|c|bem",
        Some("<div className=\"block\">${0}</div>\n<!-- /.block -->"),
    ),
    (
        "jsx",
        "div.block[!class]|bem|t|c",
        Some("<div className=\"block\">${0}</div>\n<!-- /.block -->"),
    ),
    (
        "jsx",
        "div.block[!class]|c|c|c",
        Some("<div className=\"block\">${0}</div>\n<!-- /.block -->"),
    ),
    (
        "jsx",
        "div.block[!class]|bem|bem",
        Some("<div className=\"block\">${0}</div>"),
    ),
    (
        "jsx",
        "div.block[!class]|c|t",
        Some("<div className=\"block\">${0}</div>\n<!-- /.block -->"),
    ),
    (
        "jsx",
        "div.block[!class]|t|c|bem",
        Some("<div className=\"block\">${0}</div>\n<!-- /.block -->"),
    ),
    (
        "jsx",
        "div#main.foo|bem",
        Some("<div id=\"main\" className=\"foo\">${0}</div>"),
    ),
    (
        "jsx",
        "div#main.foo|c",
        Some("<div id=\"main\" className=\"foo\">${0}</div>\n<!-- /#main.foo -->"),
    ),
    (
        "jsx",
        "div#main.foo|t",
        Some("<div id=\"main\" className=\"foo\">${0}</div>"),
    ),
    (
        "jsx",
        "div#main.foo|bem|c",
        Some("<div id=\"main\" className=\"foo\">${0}</div>\n<!-- /#main.foo -->"),
    ),
    (
        "jsx",
        "div#main.foo|c|bem",
        Some("<div id=\"main\" className=\"foo\">${0}</div>\n<!-- /#main.foo -->"),
    ),
    (
        "jsx",
        "div#main.foo|bem|t|c",
        Some("<div id=\"main\" className=\"foo\">${0}</div>\n<!-- /#main.foo -->"),
    ),
    (
        "jsx",
        "div#main.foo|c|c|c",
        Some("<div id=\"main\" className=\"foo\">${0}</div>\n<!-- /#main.foo -->"),
    ),
    (
        "jsx",
        "div#main.foo|bem|bem",
        Some("<div id=\"main\" className=\"foo\">${0}</div>"),
    ),
    (
        "jsx",
        "div#main.foo|c|t",
        Some("<div id=\"main\" className=\"foo\">${0}</div>\n<!-- /#main.foo -->"),
    ),
    (
        "jsx",
        "div#main.foo|t|c|bem",
        Some("<div id=\"main\" className=\"foo\">${0}</div>\n<!-- /#main.foo -->"),
    ),
    (
        "jsx",
        "div#main[ID=upper]|bem",
        Some("<div id=\"main\" ID=\"upper\">${0}</div>"),
    ),
    (
        "jsx",
        "div#main[ID=upper]|c",
        Some("<div id=\"main\" ID=\"upper\">${0}</div>\n<!-- /#upper -->"),
    ),
    (
        "jsx",
        "div#main[ID=upper]|t",
        Some("<div id=\"main\" ID=\"upper\">${0}</div>"),
    ),
    (
        "jsx",
        "div#main[ID=upper]|bem|c",
        Some("<div id=\"main\" ID=\"upper\">${0}</div>\n<!-- /#upper -->"),
    ),
    (
        "jsx",
        "div#main[ID=upper]|c|bem",
        Some("<div id=\"main\" ID=\"upper\">${0}</div>\n<!-- /#upper -->"),
    ),
    (
        "jsx",
        "div#main[ID=upper]|bem|t|c",
        Some("<div id=\"main\" ID=\"upper\">${0}</div>\n<!-- /#upper -->"),
    ),
    (
        "jsx",
        "div#main[ID=upper]|c|c|c",
        Some("<div id=\"main\" ID=\"upper\">${0}</div>\n<!-- /#upper -->"),
    ),
    (
        "jsx",
        "div#main[ID=upper]|bem|bem",
        Some("<div id=\"main\" ID=\"upper\">${0}</div>"),
    ),
    (
        "jsx",
        "div#main[ID=upper]|c|t",
        Some("<div id=\"main\" ID=\"upper\">${0}</div>\n<!-- /#upper -->"),
    ),
    (
        "jsx",
        "div#main[ID=upper]|t|c|bem",
        Some("<div id=\"main\" ID=\"upper\">${0}</div>\n<!-- /#upper -->"),
    ),
    (
        "jsx",
        "div#main[ID]|bem",
        Some("<div id=\"main\" ID=\"${1}\">${0}</div>"),
    ),
    (
        "jsx",
        "div#main[ID]|c",
        Some("<div id=\"main\" ID=\"${1}\">${0}</div>\n<!-- /#main -->"),
    ),
    (
        "jsx",
        "div#main[ID]|t",
        Some("<div id=\"main\" ID=\"${1}\">${0}</div>"),
    ),
    (
        "jsx",
        "div#main[ID]|bem|c",
        Some("<div id=\"main\" ID=\"${1}\">${0}</div>\n<!-- /#main -->"),
    ),
    (
        "jsx",
        "div#main[ID]|c|bem",
        Some("<div id=\"main\" ID=\"${1}\">${0}</div>\n<!-- /#main -->"),
    ),
    (
        "jsx",
        "div#main[ID]|bem|t|c",
        Some("<div id=\"main\" ID=\"${1}\">${0}</div>\n<!-- /#main -->"),
    ),
    (
        "jsx",
        "div#main[ID]|c|c|c",
        Some("<div id=\"main\" ID=\"${1}\">${0}</div>\n<!-- /#main -->"),
    ),
    (
        "jsx",
        "div#main[ID]|bem|bem",
        Some("<div id=\"main\" ID=\"${1}\">${0}</div>"),
    ),
    (
        "jsx",
        "div#main[ID]|c|t",
        Some("<div id=\"main\" ID=\"${1}\">${0}</div>\n<!-- /#main -->"),
    ),
    (
        "jsx",
        "div#main[ID]|t|c|bem",
        Some("<div id=\"main\" ID=\"${1}\">${0}</div>\n<!-- /#main -->"),
    ),
    (
        "jsx",
        "div.ID[class=\"\"]|bem",
        Some("<div className=\"ID\">${0}</div>"),
    ),
    (
        "jsx",
        "div.ID[class=\"\"]|c",
        Some("<div className=\"ID \">${0}</div>\n<!-- /.ID  -->"),
    ),
    (
        "jsx",
        "div.ID[class=\"\"]|t",
        Some("<div className=\"ID \">${0}</div>"),
    ),
    (
        "jsx",
        "div.ID[class=\"\"]|bem|c",
        Some("<div className=\"ID\">${0}</div>\n<!-- /.ID -->"),
    ),
    (
        "jsx",
        "div.ID[class=\"\"]|c|bem",
        Some("<div className=\"ID\">${0}</div>\n<!-- /.ID -->"),
    ),
    (
        "jsx",
        "div.ID[class=\"\"]|bem|t|c",
        Some("<div className=\"ID\">${0}</div>\n<!-- /.ID -->"),
    ),
    (
        "jsx",
        "div.ID[class=\"\"]|c|c|c",
        Some("<div className=\"ID \">${0}</div>\n<!-- /.ID  -->"),
    ),
    (
        "jsx",
        "div.ID[class=\"\"]|bem|bem",
        Some("<div className=\"ID\">${0}</div>"),
    ),
    (
        "jsx",
        "div.ID[class=\"\"]|c|t",
        Some("<div className=\"ID \">${0}</div>\n<!-- /.ID  -->"),
    ),
    (
        "jsx",
        "div.ID[class=\"\"]|t|c|bem",
        Some("<div className=\"ID\">${0}</div>\n<!-- /.ID -->"),
    ),
    (
        "jsx",
        "div.[id=\"\"]|bem",
        Some("<div className=\"${1}\" id=\"\">${0}</div>"),
    ),
    (
        "jsx",
        "div.[id=\"\"]|c",
        Some("<div className=\"${1}\" id=\"\">${0}</div>\n<!-- /# -->"),
    ),
    (
        "jsx",
        "div.[id=\"\"]|t",
        Some("<div className=\"${1}\" id=\"\">${0}</div>"),
    ),
    (
        "jsx",
        "div.[id=\"\"]|bem|c",
        Some("<div className=\"${1}\" id=\"\">${0}</div>\n<!-- /# -->"),
    ),
    (
        "jsx",
        "div.[id=\"\"]|c|bem",
        Some("<div className=\"${1}\" id=\"\">${0}</div>\n<!-- /# -->"),
    ),
    (
        "jsx",
        "div.[id=\"\"]|bem|t|c",
        Some("<div className=\"${1}\" id=\"\">${0}</div>\n<!-- /# -->"),
    ),
    (
        "jsx",
        "div.[id=\"\"]|c|c|c",
        Some("<div className=\"${1}\" id=\"\">${0}</div>\n<!-- /# -->"),
    ),
    (
        "jsx",
        "div.[id=\"\"]|bem|bem",
        Some("<div className=\"${1}\" id=\"\">${0}</div>"),
    ),
    (
        "jsx",
        "div.[id=\"\"]|c|t",
        Some("<div className=\"${1}\" id=\"\">${0}</div>\n<!-- /# -->"),
    ),
    (
        "jsx",
        "div.[id=\"\"]|t|c|bem",
        Some("<div className=\"${1}\" id=\"\">${0}</div>\n<!-- /# -->"),
    ),
    (
        "jsx",
        "input#main.foo|bem",
        Some("<input type=\"${2:text}\" id=\"main\" className=\"foo\">"),
    ),
    (
        "jsx",
        "input#main.foo|c",
        Some("<input type=\"${2:text}\" id=\"main\" className=\"foo\">"),
    ),
    (
        "jsx",
        "input#main.foo|t",
        Some("<input type=\"${2:text}\" id=\"main\" className=\"foo\">"),
    ),
    (
        "jsx",
        "input#main.foo|bem|c",
        Some("<input type=\"${2:text}\" id=\"main\" className=\"foo\">"),
    ),
    (
        "jsx",
        "input#main.foo|c|bem",
        Some("<input type=\"${2:text}\" id=\"main\" className=\"foo\">"),
    ),
    (
        "jsx",
        "input#main.foo|bem|t|c",
        Some("<input type=\"${2:text}\" id=\"main\" className=\"foo\">"),
    ),
    (
        "jsx",
        "input#main.foo|c|c|c",
        Some("<input type=\"${2:text}\" id=\"main\" className=\"foo\">"),
    ),
    (
        "jsx",
        "input#main.foo|bem|bem",
        Some("<input type=\"${2:text}\" id=\"main\" className=\"foo\">"),
    ),
    (
        "jsx",
        "input#main.foo|c|t",
        Some("<input type=\"${2:text}\" id=\"main\" className=\"foo\">"),
    ),
    (
        "jsx",
        "input#main.foo|t|c|bem",
        Some("<input type=\"${2:text}\" id=\"main\" className=\"foo\">"),
    ),
    (
        "jsx",
        "div.block>span.-element+span._modifier|bem",
        Some(
            "<div className=\"block\"><span className=\"block__element\">${1}</span><span className=\"block block_modifier\">${0}</span></div>",
        ),
    ),
    (
        "jsx",
        "div.block>span.-element+span._modifier|c",
        Some(
            "<div className=\"block\"><span className=\"-element\">${1}</span>\n\t<!-- /.-element --><span className=\"_modifier\">${0}</span>\n\t<!-- /._modifier --></div>\n<!-- /.block -->",
        ),
    ),
    (
        "jsx",
        "div.block>span.-element+span._modifier|t",
        Some(
            "<div className=\"block\"><span className=\"-element\">${1}</span><span className=\"_modifier\">${0}</span></div>",
        ),
    ),
    (
        "jsx",
        "div.block>span.-element+span._modifier|bem|c",
        Some(
            "<div className=\"block\"><span className=\"block__element\">${1}</span>\n\t<!-- /.block__element --><span className=\"block block_modifier\">${0}</span>\n\t<!-- /.block block_modifier --></div>\n<!-- /.block -->",
        ),
    ),
    (
        "jsx",
        "div.block>span.-element+span._modifier|c|bem",
        Some(
            "<div className=\"block\"><span className=\"block__element\">${1}</span>\n\t<!-- /.block__element --><span className=\"block block_modifier\">${0}</span>\n\t<!-- /.block block_modifier --></div>\n<!-- /.block -->",
        ),
    ),
    (
        "jsx",
        "div.block>span.-element+span._modifier|bem|t|c",
        Some(
            "<div className=\"block\"><span className=\"block__element\">${1}</span>\n\t<!-- /.block__element --><span className=\"block block_modifier\">${0}</span>\n\t<!-- /.block block_modifier --></div>\n<!-- /.block -->",
        ),
    ),
    (
        "jsx",
        "div.block>span.-element+span._modifier|c|c|c",
        Some(
            "<div className=\"block\"><span className=\"-element\">${1}</span>\n\t<!-- /.-element --><span className=\"_modifier\">${0}</span>\n\t<!-- /._modifier --></div>\n<!-- /.block -->",
        ),
    ),
    (
        "jsx",
        "div.block>span.-element+span._modifier|bem|bem",
        Some(
            "<div className=\"block\"><span className=\"block__element\">${1}</span><span className=\"block block_modifier\">${0}</span></div>",
        ),
    ),
    (
        "jsx",
        "div.block>span.-element+span._modifier|c|t",
        Some(
            "<div className=\"block\"><span className=\"-element\">${1}</span>\n\t<!-- /.-element --><span className=\"_modifier\">${0}</span>\n\t<!-- /._modifier --></div>\n<!-- /.block -->",
        ),
    ),
    (
        "jsx",
        "div.block>span.-element+span._modifier|t|c|bem",
        Some(
            "<div className=\"block\"><span className=\"block__element\">${1}</span>\n\t<!-- /.block__element --><span className=\"block block_modifier\">${0}</span>\n\t<!-- /.block block_modifier --></div>\n<!-- /.block -->",
        ),
    ),
    (
        "jsx",
        "ul.block>li.-item*3|bem",
        Some(
            "<ul className=\"block\">\n\t<li className=\"block__item\">${1}</li>\n\t<li className=\"block__item\">${2}</li>\n\t<li className=\"block__item\">${0}</li>\n</ul>",
        ),
    ),
    (
        "jsx",
        "ul.block>li.-item*3|c",
        Some(
            "<ul className=\"block\">\n\t<li className=\"-item\">${1}</li>\n\t<!-- /.-item -->\n\t<li className=\"-item\">${2}</li>\n\t<!-- /.-item -->\n\t<li className=\"-item\">${0}</li>\n\t<!-- /.-item -->\n</ul>\n<!-- /.block -->",
        ),
    ),
    (
        "jsx",
        "ul.block>li.-item*3|t",
        Some(
            "<ul className=\"block\">\n\t<li className=\"-item\">${1}</li>\n\t<li className=\"-item\">${2}</li>\n\t<li className=\"-item\">${0}</li>\n</ul>",
        ),
    ),
    (
        "jsx",
        "ul.block>li.-item*3|bem|c",
        Some(
            "<ul className=\"block\">\n\t<li className=\"block__item\">${1}</li>\n\t<!-- /.block__item -->\n\t<li className=\"block__item\">${2}</li>\n\t<!-- /.block__item -->\n\t<li className=\"block__item\">${0}</li>\n\t<!-- /.block__item -->\n</ul>\n<!-- /.block -->",
        ),
    ),
    (
        "jsx",
        "ul.block>li.-item*3|c|bem",
        Some(
            "<ul className=\"block\">\n\t<li className=\"block__item\">${1}</li>\n\t<!-- /.block__item -->\n\t<li className=\"block__item\">${2}</li>\n\t<!-- /.block__item -->\n\t<li className=\"block__item\">${0}</li>\n\t<!-- /.block__item -->\n</ul>\n<!-- /.block -->",
        ),
    ),
    (
        "jsx",
        "ul.block>li.-item*3|bem|t|c",
        Some(
            "<ul className=\"block\">\n\t<li className=\"block__item\">${1}</li>\n\t<!-- /.block__item -->\n\t<li className=\"block__item\">${2}</li>\n\t<!-- /.block__item -->\n\t<li className=\"block__item\">${0}</li>\n\t<!-- /.block__item -->\n</ul>\n<!-- /.block -->",
        ),
    ),
    (
        "jsx",
        "ul.block>li.-item*3|c|c|c",
        Some(
            "<ul className=\"block\">\n\t<li className=\"-item\">${1}</li>\n\t<!-- /.-item -->\n\t<li className=\"-item\">${2}</li>\n\t<!-- /.-item -->\n\t<li className=\"-item\">${0}</li>\n\t<!-- /.-item -->\n</ul>\n<!-- /.block -->",
        ),
    ),
    (
        "jsx",
        "ul.block>li.-item*3|bem|bem",
        Some(
            "<ul className=\"block\">\n\t<li className=\"block__item\">${1}</li>\n\t<li className=\"block__item\">${2}</li>\n\t<li className=\"block__item\">${0}</li>\n</ul>",
        ),
    ),
    (
        "jsx",
        "ul.block>li.-item*3|c|t",
        Some(
            "<ul className=\"block\">\n\t<li className=\"-item\">${1}</li>\n\t<!-- /.-item -->\n\t<li className=\"-item\">${2}</li>\n\t<!-- /.-item -->\n\t<li className=\"-item\">${0}</li>\n\t<!-- /.-item -->\n</ul>\n<!-- /.block -->",
        ),
    ),
    (
        "jsx",
        "ul.block>li.-item*3|t|c|bem",
        Some(
            "<ul className=\"block\">\n\t<li className=\"block__item\">${1}</li>\n\t<!-- /.block__item -->\n\t<li className=\"block__item\">${2}</li>\n\t<!-- /.block__item -->\n\t<li className=\"block__item\">${0}</li>\n\t<!-- /.block__item -->\n</ul>\n<!-- /.block -->",
        ),
    ),
    (
        "jsx",
        "div..block>.-element_modifier|bem",
        Some(
            "<div styleName={styles.block}>\n\t<div className=\"block__element block__element_modifier\">${0}</div>\n</div>",
        ),
    ),
    (
        "jsx",
        "div..block>.-element_modifier|c",
        Some(
            "<div styleName={styles.block}>\n\t<div className=\"-element_modifier\">${0}</div>\n\t<!-- /.-element_modifier -->\n</div>\n<!-- /.block -->",
        ),
    ),
    (
        "jsx",
        "div..block>.-element_modifier|t",
        Some(
            "<div styleName={styles.block}>\n\t<div className=\"-element_modifier\">${0}</div>\n</div>",
        ),
    ),
    (
        "jsx",
        "div..block>.-element_modifier|bem|c",
        Some(
            "<div styleName={styles.block}>\n\t<div className=\"block__element block__element_modifier\">${0}</div>\n\t<!-- /.block__element block__element_modifier -->\n</div>\n<!-- /.block -->",
        ),
    ),
    (
        "jsx",
        "div..block>.-element_modifier|c|bem",
        Some(
            "<div styleName={styles.block}>\n\t<div className=\"block__element block__element_modifier\">${0}</div>\n\t<!-- /.block__element block__element_modifier -->\n</div>\n<!-- /.block -->",
        ),
    ),
    (
        "jsx",
        "div..block>.-element_modifier|bem|t|c",
        Some(
            "<div styleName={styles.block}>\n\t<div className=\"block__element block__element_modifier\">${0}</div>\n\t<!-- /.block__element block__element_modifier -->\n</div>\n<!-- /.block -->",
        ),
    ),
    (
        "jsx",
        "div..block>.-element_modifier|c|c|c",
        Some(
            "<div styleName={styles.block}>\n\t<div className=\"-element_modifier\">${0}</div>\n\t<!-- /.-element_modifier -->\n</div>\n<!-- /.block -->",
        ),
    ),
    (
        "jsx",
        "div..block>.-element_modifier|bem|bem",
        Some(
            "<div styleName={styles.block}>\n\t<div className=\"block__element block__element_modifier\">${0}</div>\n</div>",
        ),
    ),
    (
        "jsx",
        "div..block>.-element_modifier|c|t",
        Some(
            "<div styleName={styles.block}>\n\t<div className=\"-element_modifier\">${0}</div>\n\t<!-- /.-element_modifier -->\n</div>\n<!-- /.block -->",
        ),
    ),
    (
        "jsx",
        "div..block>.-element_modifier|t|c|bem",
        Some(
            "<div styleName={styles.block}>\n\t<div className=\"block__element block__element_modifier\">${0}</div>\n\t<!-- /.block__element block__element_modifier -->\n</div>\n<!-- /.block -->",
        ),
    ),
    (
        "jsx",
        "div[class=\"block﻿foo\"]|bem",
        Some("<div className=\"block foo\">${0}</div>"),
    ),
    (
        "jsx",
        "div[class=\"block﻿foo\"]|c",
        Some("<div className=\"block﻿foo\">${0}</div>\n<!-- /.block﻿foo -->"),
    ),
    (
        "jsx",
        "div[class=\"block﻿foo\"]|t",
        Some("<div className=\"block﻿foo\">${0}</div>"),
    ),
    (
        "jsx",
        "div[class=\"block﻿foo\"]|bem|c",
        Some("<div className=\"block foo\">${0}</div>\n<!-- /.block foo -->"),
    ),
    (
        "jsx",
        "div[class=\"block﻿foo\"]|c|bem",
        Some("<div className=\"block foo\">${0}</div>\n<!-- /.block foo -->"),
    ),
    (
        "jsx",
        "div[class=\"block﻿foo\"]|bem|t|c",
        Some("<div className=\"block foo\">${0}</div>\n<!-- /.block foo -->"),
    ),
    (
        "jsx",
        "div[class=\"block﻿foo\"]|c|c|c",
        Some("<div className=\"block﻿foo\">${0}</div>\n<!-- /.block﻿foo -->"),
    ),
    (
        "jsx",
        "div[class=\"block﻿foo\"]|bem|bem",
        Some("<div className=\"block foo\">${0}</div>"),
    ),
    (
        "jsx",
        "div[class=\"block﻿foo\"]|c|t",
        Some("<div className=\"block﻿foo\">${0}</div>\n<!-- /.block﻿foo -->"),
    ),
    (
        "jsx",
        "div[class=\"block﻿foo\"]|t|c|bem",
        Some("<div className=\"block foo\">${0}</div>\n<!-- /.block foo -->"),
    ),
    (
        "jsx",
        "div[class=\"blockfoo\"]|bem",
        Some("<div className=\"blockfoo\">${0}</div>"),
    ),
    (
        "jsx",
        "div[class=\"blockfoo\"]|c",
        Some("<div className=\"blockfoo\">${0}</div>\n<!-- /.blockfoo -->"),
    ),
    (
        "jsx",
        "div[class=\"blockfoo\"]|t",
        Some("<div className=\"blockfoo\">${0}</div>"),
    ),
    (
        "jsx",
        "div[class=\"blockfoo\"]|bem|c",
        Some("<div className=\"blockfoo\">${0}</div>\n<!-- /.blockfoo -->"),
    ),
    (
        "jsx",
        "div[class=\"blockfoo\"]|c|bem",
        Some("<div className=\"blockfoo\">${0}</div>\n<!-- /.blockfoo -->"),
    ),
    (
        "jsx",
        "div[class=\"blockfoo\"]|bem|t|c",
        Some("<div className=\"blockfoo\">${0}</div>\n<!-- /.blockfoo -->"),
    ),
    (
        "jsx",
        "div[class=\"blockfoo\"]|c|c|c",
        Some("<div className=\"blockfoo\">${0}</div>\n<!-- /.blockfoo -->"),
    ),
    (
        "jsx",
        "div[class=\"blockfoo\"]|bem|bem",
        Some("<div className=\"blockfoo\">${0}</div>"),
    ),
    (
        "jsx",
        "div[class=\"blockfoo\"]|c|t",
        Some("<div className=\"blockfoo\">${0}</div>\n<!-- /.blockfoo -->"),
    ),
    (
        "jsx",
        "div[class=\"blockfoo\"]|t|c|bem",
        Some("<div className=\"blockfoo\">${0}</div>\n<!-- /.blockfoo -->"),
    ),
    (
        "jsx",
        "div#${1}.block|bem",
        Some("<div id=\"1\" className=\"block\">1</div>"),
    ),
    (
        "jsx",
        "div#${1}.block|c",
        Some("<div id=\"1\" className=\"block\">1</div>\n<!-- /#1.block -->"),
    ),
    (
        "jsx",
        "div#${1}.block|t",
        Some("<div id=\"1\" className=\"block\">1</div>"),
    ),
    (
        "jsx",
        "div#${1}.block|bem|c",
        Some("<div id=\"1\" className=\"block\">1</div>\n<!-- /#1.block -->"),
    ),
    (
        "jsx",
        "div#${1}.block|c|bem",
        Some("<div id=\"1\" className=\"block\">1</div>\n<!-- /#1.block -->"),
    ),
    (
        "jsx",
        "div#${1}.block|bem|t|c",
        Some("<div id=\"1\" className=\"block\">1</div>\n<!-- /#1.block -->"),
    ),
    (
        "jsx",
        "div#${1}.block|c|c|c",
        Some("<div id=\"1\" className=\"block\">1</div>\n<!-- /#1.block -->"),
    ),
    (
        "jsx",
        "div#${1}.block|bem|bem",
        Some("<div id=\"1\" className=\"block\">1</div>"),
    ),
    (
        "jsx",
        "div#${1}.block|c|t",
        Some("<div id=\"1\" className=\"block\">1</div>\n<!-- /#1.block -->"),
    ),
    (
        "jsx",
        "div#${1}.block|t|c|bem",
        Some("<div id=\"1\" className=\"block\">1</div>\n<!-- /#1.block -->"),
    ),
    (
        "jsx",
        "div#main>span.foo|bem",
        Some("<div id=\"main\"><span className=\"foo\">${0}</span></div>"),
    ),
    (
        "jsx",
        "div#main>span.foo|c",
        Some(
            "<div id=\"main\"><span className=\"foo\">${0}</span>\n\t<!-- /.foo --></div>\n<!-- /#main -->",
        ),
    ),
    (
        "jsx",
        "div#main>span.foo|t",
        Some("<div id=\"main\"><span className=\"foo\">${0}</span></div>"),
    ),
    (
        "jsx",
        "div#main>span.foo|bem|c",
        Some(
            "<div id=\"main\"><span className=\"foo\">${0}</span>\n\t<!-- /.foo --></div>\n<!-- /#main -->",
        ),
    ),
    (
        "jsx",
        "div#main>span.foo|c|bem",
        Some(
            "<div id=\"main\"><span className=\"foo\">${0}</span>\n\t<!-- /.foo --></div>\n<!-- /#main -->",
        ),
    ),
    (
        "jsx",
        "div#main>span.foo|bem|t|c",
        Some(
            "<div id=\"main\"><span className=\"foo\">${0}</span>\n\t<!-- /.foo --></div>\n<!-- /#main -->",
        ),
    ),
    (
        "jsx",
        "div#main>span.foo|c|c|c",
        Some(
            "<div id=\"main\"><span className=\"foo\">${0}</span>\n\t<!-- /.foo --></div>\n<!-- /#main -->",
        ),
    ),
    (
        "jsx",
        "div#main>span.foo|bem|bem",
        Some("<div id=\"main\"><span className=\"foo\">${0}</span></div>"),
    ),
    (
        "jsx",
        "div#main>span.foo|c|t",
        Some(
            "<div id=\"main\"><span className=\"foo\">${0}</span>\n\t<!-- /.foo --></div>\n<!-- /#main -->",
        ),
    ),
    (
        "jsx",
        "div#main>span.foo|t|c|bem",
        Some(
            "<div id=\"main\"><span className=\"foo\">${0}</span>\n\t<!-- /.foo --></div>\n<!-- /#main -->",
        ),
    ),
];
