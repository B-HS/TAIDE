const CASES: &[(&str, u32, &str, Option<&str>)] = &[
    (
        "html",
        1,
        "lorem",
        Some(
            "Lorem ipsum dolor sit amet consectetur adipisicing elit. Accusamus eum, sint id, dolores laborum quam ex amet molestias recusandae sequi porro aperiam rem consequuntur? Tempore accusamus voluptatem unde voluptatibus nemo?",
        ),
    ),
    (
        "html",
        1,
        "lorem|bem|c",
        Some(
            "Lorem ipsum dolor sit amet consectetur adipisicing elit. Accusamus eum, sint id, dolores laborum quam ex amet molestias recusandae sequi porro aperiam rem consequuntur? Tempore accusamus voluptatem unde voluptatibus nemo?",
        ),
    ),
    ("html", 1, "lorem0", Some("Lorem.")),
    ("html", 1, "lorem0|bem|c", Some("Lorem.")),
    ("html", 1, "lorem1", Some("Lorem.")),
    ("html", 1, "lorem1|bem|c", Some("Lorem.")),
    ("html", 1, "lorem2", Some("Lorem, ipsum.")),
    ("html", 1, "lorem2|bem|c", Some("Lorem, ipsum.")),
    (
        "html",
        1,
        "lorem8",
        Some("Lorem ipsum dolor sit amet consectetur adipisicing elit."),
    ),
    (
        "html",
        1,
        "lorem8|bem|c",
        Some("Lorem ipsum dolor sit amet consectetur adipisicing elit."),
    ),
    (
        "html",
        1,
        "lorem9",
        Some("Lorem ipsum dolor sit amet consectetur adipisicing elit. Accusamus?"),
    ),
    (
        "html",
        1,
        "lorem9|bem|c",
        Some("Lorem ipsum dolor sit amet consectetur adipisicing elit. Accusamus?"),
    ),
    (
        "html",
        1,
        "lorem30",
        Some(
            "Lorem ipsum dolor sit amet consectetur adipisicing elit. Accusamus eum, sint id, dolores laborum quam ex amet molestias recusandae sequi porro aperiam rem consequuntur? Tempore accusamus voluptatem unde voluptatibus nemo?",
        ),
    ),
    (
        "html",
        1,
        "lorem30|bem|c",
        Some(
            "Lorem ipsum dolor sit amet consectetur adipisicing elit. Accusamus eum, sint id, dolores laborum quam ex amet molestias recusandae sequi porro aperiam rem consequuntur? Tempore accusamus voluptatem unde voluptatibus nemo?",
        ),
    ),
    (
        "html",
        1,
        "lorem100",
        Some(
            "Lorem ipsum dolor sit amet consectetur adipisicing elit. Accusamus eum, sint id, dolores laborum quam ex amet molestias recusandae sequi porro aperiam rem consequuntur? Tempore accusamus voluptatem unde voluptatibus nemo mollitia ipsum necessitatibus porro. Maxime, commodi ut alias, deleniti nihil sint consequuntur expedita minima vero, molestias asperiores necessitatibus laborum! Adipisci, tempore sit. Perspiciatis sunt corrupti fugiat ipsa voluptate laboriosam dolore eius impedit soluta facilis repellat ad quas quia ducimus eum, amet molestiae similique suscipit id veritatis veniam a aliquam possimus. Maiores recusandae assumenda exercitationem, quidem atque, consectetur nostrum natus sed in repudiandae, illum quia. Ipsa distinctio non nulla repellendus perferendis.",
        ),
    ),
    (
        "html",
        1,
        "lorem100|bem|c",
        Some(
            "Lorem ipsum dolor sit amet consectetur adipisicing elit. Accusamus eum, sint id, dolores laborum quam ex amet molestias recusandae sequi porro aperiam rem consequuntur? Tempore accusamus voluptatem unde voluptatibus nemo mollitia ipsum necessitatibus porro. Maxime, commodi ut alias, deleniti nihil sint consequuntur expedita minima vero, molestias asperiores necessitatibus laborum! Adipisci, tempore sit. Perspiciatis sunt corrupti fugiat ipsa voluptate laboriosam dolore eius impedit soluta facilis repellat ad quas quia ducimus eum, amet molestiae similique suscipit id veritatis veniam a aliquam possimus. Maiores recusandae assumenda exercitationem, quidem atque, consectetur nostrum natus sed in repudiandae, illum quia. Ipsa distinctio non nulla repellendus perferendis.",
        ),
    ),
    (
        "html",
        1,
        "lorem5-20",
        Some("Lorem ipsum dolor sit amet consectetur adipisicing elit."),
    ),
    (
        "html",
        1,
        "lorem5-20|bem|c",
        Some("Lorem ipsum dolor sit amet consectetur adipisicing elit."),
    ),
    (
        "html",
        1,
        "lorem20-5",
        Some(
            "Lorem ipsum dolor sit amet consectetur adipisicing elit. Accusamus eum sint id dolores laborum quam ex amet, molestias recusandae sequi!",
        ),
    ),
    (
        "html",
        1,
        "lorem20-5|bem|c",
        Some(
            "Lorem ipsum dolor sit amet consectetur adipisicing elit. Accusamus eum sint id dolores laborum quam ex amet, molestias recusandae sequi!",
        ),
    ),
    ("html", 1, "lorem3-", Some("Lorem, ipsum dolor.")),
    ("html", 1, "lorem3-|bem|c", Some("Lorem, ipsum dolor.")),
    (
        "html",
        1,
        "loremru12",
        Some(
            "Далеко-далеко за словесными горами в стране гласных и согласных живут рыбные тексты. Предупредила, языкового?",
        ),
    ),
    (
        "html",
        1,
        "loremru12|bem|c",
        Some(
            "Далеко-далеко за словесными горами в стране гласных и согласных живут рыбные тексты. Предупредила, языкового?",
        ),
    ),
    (
        "html",
        1,
        "loremsp40",
        Some(
            "Mujer uno dolor más de poder mismo si. Cosas mirar, sintió vos, dolores laboral manantial ex entero molestias recusado séquito ventana síndrome lejos afortunado? Temporal cosas voluntad falda voluminoso remo computador aparte necesariamente ventana. Anemia comodidad o alias deleznable nihilista.",
        ),
    ),
    (
        "html",
        1,
        "loremsp40|bem|c",
        Some(
            "Mujer uno dolor más de poder mismo si. Cosas mirar, sintió vos, dolores laboral manantial ex entero molestias recusado séquito ventana síndrome lejos afortunado? Temporal cosas voluntad falda voluminoso remo computador aparte necesariamente ventana. Anemia comodidad o alias deleznable nihilista.",
        ),
    ),
    (
        "html",
        1,
        "loremfr12",
        Some("Lorem ipsum dolor sit amet consectetur adipisicing elit. Accusamus eum sint id?"),
    ),
    (
        "html",
        1,
        "loremfr12|bem|c",
        Some("Lorem ipsum dolor sit amet consectetur adipisicing elit. Accusamus eum sint id?"),
    ),
    (
        "html",
        1,
        "LOREMRU12",
        Some("Lorem ipsum dolor sit amet consectetur adipisicing elit. Accusamus eum sint id?"),
    ),
    (
        "html",
        1,
        "LOREMRU12|bem|c",
        Some("Lorem ipsum dolor sit amet consectetur adipisicing elit. Accusamus eum sint id?"),
    ),
    ("html", 1, "loremru1", Some("Далеко-далеко.")),
    ("html", 1, "loremru1|bem|c", Some("Далеко-далеко.")),
    ("html", 1, "loremsp1", Some("Mujer.")),
    ("html", 1, "loremsp1|bem|c", Some("Mujer.")),
    (
        "html",
        1,
        "lorem0008",
        Some("Lorem ipsum dolor sit amet consectetur adipisicing elit."),
    ),
    (
        "html",
        1,
        "lorem0008|bem|c",
        Some("Lorem ipsum dolor sit amet consectetur adipisicing elit."),
    ),
    (
        "html",
        1,
        "lorem-8",
        Some(
            "Lorem ipsum dolor sit amet consectetur adipisicing elit. Accusamus eum, sint id, dolores laborum quam ex amet molestias recusandae sequi porro aperiam rem consequuntur? Tempore accusamus voluptatem unde voluptatibus nemo?",
        ),
    ),
    (
        "html",
        1,
        "lorem-8|bem|c",
        Some(
            "Lorem ipsum dolor sit amet consectetur adipisicing elit. Accusamus eum, sint id, dolores laborum quam ex amet molestias recusandae sequi porro aperiam rem consequuntur? Tempore accusamus voluptatem unde voluptatibus nemo?",
        ),
    ),
    (
        "html",
        1,
        "lorem-0",
        Some(
            "Lorem ipsum dolor sit amet consectetur adipisicing elit. Accusamus eum, sint id, dolores laborum quam ex amet molestias recusandae sequi porro aperiam rem consequuntur? Tempore accusamus voluptatem unde voluptatibus nemo?",
        ),
    ),
    (
        "html",
        1,
        "lorem-0|bem|c",
        Some(
            "Lorem ipsum dolor sit amet consectetur adipisicing elit. Accusamus eum, sint id, dolores laborum quam ex amet molestias recusandae sequi porro aperiam rem consequuntur? Tempore accusamus voluptatem unde voluptatibus nemo?",
        ),
    ),
    (
        "html",
        1,
        "lorem*3",
        Some(
            "Lorem ipsum dolor sit amet consectetur adipisicing elit. Accusamus eum, sint id, dolores laborum quam ex amet molestias recusandae sequi porro aperiam rem consequuntur? Tempore accusamus voluptatem unde voluptatibus nemo?\nSapiente quis reiciendis maxime commodi ut alias deleniti nihil sint consequuntur expedita minima vero molestias asperiores necessitatibus laborum eos sequi magnam eveniet, unde nobis adipisci tempore, sit laudantium facilis? Corrupti.\nLaboriosam dolore eius impedit soluta facilis repellat ad quas quia ducimus eum. Suscipit id veritatis veniam, a aliquam possimus laborum atque nemo culpa maiores recusandae assumenda exercitationem quidem consectetur nostrum.",
        ),
    ),
    (
        "html",
        1,
        "lorem*3|bem|c",
        Some(
            "Lorem ipsum dolor sit amet consectetur adipisicing elit. Accusamus eum, sint id, dolores laborum quam ex amet molestias recusandae sequi porro aperiam rem consequuntur? Tempore accusamus voluptatem unde voluptatibus nemo?\nSapiente quis reiciendis maxime commodi ut alias deleniti nihil sint consequuntur expedita minima vero molestias asperiores necessitatibus laborum eos sequi magnam eveniet, unde nobis adipisci tempore, sit laudantium facilis? Corrupti.\nLaboriosam dolore eius impedit soluta facilis repellat ad quas quia ducimus eum. Suscipit id veritatis veniam, a aliquam possimus laborum atque nemo culpa maiores recusandae assumenda exercitationem quidem consectetur nostrum.",
        ),
    ),
    (
        "html",
        1,
        "lorem5*3",
        Some(
            "Lorem ipsum dolor sit amet.\nEum sint id dolores laborum!\nRecusandae sequi porro aperiam rem.",
        ),
    ),
    (
        "html",
        1,
        "lorem5*3|bem|c",
        Some(
            "Lorem ipsum dolor sit amet.\nEum sint id dolores laborum!\nRecusandae sequi porro aperiam rem.",
        ),
    ),
    (
        "html",
        1,
        "p>lorem5*3",
        Some(
            "<p><span>Lorem ipsum dolor sit amet.</span><span>Eum sint id dolores laborum!</span><span>Recusandae sequi porro aperiam rem.</span></p>",
        ),
    ),
    (
        "html",
        1,
        "p>lorem5*3|bem|c",
        Some(
            "<p><span>Lorem ipsum dolor sit amet.</span><span>Eum sint id dolores laborum!</span><span>Recusandae sequi porro aperiam rem.</span></p>",
        ),
    ),
    (
        "html",
        1,
        "p*3>lorem12",
        Some(
            "<p>Lorem ipsum dolor sit amet consectetur adipisicing elit. Accusamus eum sint id?</p>\n<p>Amet molestias recusandae sequi porro aperiam rem consequuntur totam? Praesentium, tempore accusamus.</p>\n<p>Necessitatibus porro sapiente quis reiciendis maxime. Deleniti nihil sint consequuntur expedita minima.</p>",
        ),
    ),
    (
        "html",
        1,
        "p*3>lorem12|bem|c",
        Some(
            "<p>Lorem ipsum dolor sit amet consectetur adipisicing elit. Accusamus eum sint id?</p>\n<p>Amet molestias recusandae sequi porro aperiam rem consequuntur totam? Praesentium, tempore accusamus.</p>\n<p>Necessitatibus porro sapiente quis reiciendis maxime. Deleniti nihil sint consequuntur expedita minima.</p>",
        ),
    ),
    (
        "html",
        1,
        "ul>lorem8*2",
        Some(
            "<ul>\n\t<li>Lorem ipsum dolor sit amet consectetur adipisicing elit.</li>\n\t<li>Eum sint id dolores laborum quam, ex amet?</li>\n</ul>",
        ),
    ),
    (
        "html",
        1,
        "ul>lorem8*2|bem|c",
        Some(
            "<ul>\n\t<li>Lorem ipsum dolor sit amet consectetur adipisicing elit.</li>\n\t<li>Eum sint id dolores laborum quam, ex amet?</li>\n</ul>",
        ),
    ),
    (
        "html",
        1,
        "ul*2>li>lorem10",
        Some(
            "<ul>\n\t<li>Lorem ipsum dolor sit amet consectetur adipisicing elit. Accusamus, eum?</li>\n</ul>\n<ul>\n\t<li>Amet molestias recusandae sequi porro aperiam rem consequuntur totam? Praesentium?</li>\n</ul>",
        ),
    ),
    (
        "html",
        1,
        "ul*2>li>lorem10|bem|c",
        Some(
            "<ul>\n\t<li>Lorem ipsum dolor sit amet consectetur adipisicing elit. Accusamus, eum?</li>\n</ul>\n<ul>\n\t<li>Amet molestias recusandae sequi porro aperiam rem consequuntur totam? Praesentium?</li>\n</ul>",
        ),
    ),
    (
        "html",
        1,
        "(h2{title}+p>lorem12)*2",
        Some(
            "<h2>title</h2>\n<p>Lorem ipsum dolor sit amet consectetur adipisicing elit. Accusamus eum sint id?</p>\n<h2>title</h2>\n<p>Amet molestias recusandae sequi porro aperiam rem consequuntur totam? Praesentium, tempore accusamus.</p>",
        ),
    ),
    (
        "html",
        1,
        "(h2{title}+p>lorem12)*2|bem|c",
        Some(
            "<h2>title</h2>\n<p>Lorem ipsum dolor sit amet consectetur adipisicing elit. Accusamus eum sint id?</p>\n<h2>title</h2>\n<p>Amet molestias recusandae sequi porro aperiam rem consequuntur totam? Praesentium, tempore accusamus.</p>",
        ),
    ),
    (
        "html",
        1,
        "div>lorem5>span{end}",
        Some("<div>Lorem ipsum dolor sit amet.<span>end</span></div>"),
    ),
    (
        "html",
        1,
        "div>lorem5>span{end}|bem|c",
        Some("<div>Lorem ipsum dolor sit amet.<span>end</span></div>"),
    ),
    (
        "html",
        1,
        "p>lorem5.class#id[title=ignored]",
        Some("<p>Lorem ipsum dolor sit amet.</p>"),
    ),
    (
        "html",
        1,
        "p>lorem5.class#id[title=ignored]|bem|c",
        Some("<p>Lorem ipsum dolor sit amet.</p>"),
    ),
    (
        "html",
        1,
        "div.block>p.-element>lorem12",
        Some(
            "<div class=\"block\">\n\t<p class=\"-element\">Lorem ipsum dolor sit amet consectetur adipisicing elit. Accusamus eum sint id?</p>\n</div>",
        ),
    ),
    (
        "html",
        1,
        "div.block>p.-element>lorem12|bem|c",
        Some(
            "<div class=\"block\">\n\t<p class=\"block__element\">Lorem ipsum dolor sit amet consectetur adipisicing elit. Accusamus eum sint id?</p>\n\t<!-- /.block__element -->\n</div>\n<!-- /.block -->",
        ),
    ),
    (
        "html",
        1,
        "lorem5{ignored}",
        Some("Lorem ipsum dolor sit amet."),
    ),
    (
        "html",
        1,
        "lorem5{ignored}|bem|c",
        Some("Lorem ipsum dolor sit amet."),
    ),
    (
        "html",
        1,
        "loremx5+span",
        Some("Lorem ipsum dolor sit amet.<span>${0}</span>"),
    ),
    (
        "html",
        1,
        "loremx5+span|bem|c",
        Some("Lorem ipsum dolor sit amet.<span>${0}</span>"),
    ),
    ("html", 1, "lorem5x", Some("<lorem5x>${0}</lorem5x>")),
    ("html", 1, "lorem5x|bem|c", Some("<lorem5x>${0}</lorem5x>")),
    ("html", 1, "lorem1.5", Some("Lorem.")),
    ("html", 1, "lorem1.5|bem|c", Some("Lorem.")),
    (
        "html",
        1,
        "lorem2-4-6",
        Some("<lorem2-4-6>${0}</lorem2-4-6>"),
    ),
    (
        "html",
        1,
        "lorem2-4-6|bem|c",
        Some("<lorem2-4-6>${0}</lorem2-4-6>"),
    ),
    (
        "html",
        1,
        "p>loremru12*2",
        Some(
            "<p><span>Далеко-далеко за словесными горами в стране гласных и согласных живут рыбные тексты. Предупредила, языкового?</span><span>Сбить заголовок путь безорфографичный там коварный грамматики встретил гор? Мир, страна моей.</span></p>",
        ),
    ),
    (
        "html",
        1,
        "p>loremru12*2|bem|c",
        Some(
            "<p><span>Далеко-далеко за словесными горами в стране гласных и согласных живут рыбные тексты. Предупредила, языкового?</span><span>Сбить заголовок путь безорфографичный там коварный грамматики встретил гор? Мир, страна моей.</span></p>",
        ),
    ),
    (
        "html",
        1,
        "table>loremsp9*2",
        Some(
            "<table>\n\t<tr>Mujer uno dolor más de poder mismo si. Cosas?</tr>\n\t<tr>Dolores laboral manantial, ex entero molestias recusado séquito ventana.</tr>\n</table>",
        ),
    ),
    (
        "html",
        1,
        "table>loremsp9*2|bem|c",
        Some(
            "<table>\n\t<tr>Mujer uno dolor más de poder mismo si. Cosas?</tr>\n\t<tr>Dolores laboral manantial, ex entero molestias recusado séquito ventana.</tr>\n</table>",
        ),
    ),
    (
        "html",
        42,
        "lorem",
        Some(
            "Lorem ipsum dolor sit amet consectetur adipisicing elit. Iusto accusantium eveniet libero non commodi corrupti doloremque maiores, quam aperiam magni aut dignissimos similique esse dolor voluptates! Beatae nihil repellendus reiciendis?",
        ),
    ),
    (
        "html",
        42,
        "lorem|bem|c",
        Some(
            "Lorem ipsum dolor sit amet consectetur adipisicing elit. Iusto accusantium eveniet libero non commodi corrupti doloremque maiores, quam aperiam magni aut dignissimos similique esse dolor voluptates! Beatae nihil repellendus reiciendis?",
        ),
    ),
    ("html", 42, "lorem0", Some("Lorem.")),
    ("html", 42, "lorem0|bem|c", Some("Lorem.")),
    ("html", 42, "lorem1", Some("Lorem.")),
    ("html", 42, "lorem1|bem|c", Some("Lorem.")),
    ("html", 42, "lorem2", Some("Lorem, ipsum.")),
    ("html", 42, "lorem2|bem|c", Some("Lorem, ipsum.")),
    (
        "html",
        42,
        "lorem8",
        Some("Lorem ipsum dolor sit amet consectetur adipisicing elit."),
    ),
    (
        "html",
        42,
        "lorem8|bem|c",
        Some("Lorem ipsum dolor sit amet consectetur adipisicing elit."),
    ),
    (
        "html",
        42,
        "lorem9",
        Some("Lorem ipsum dolor sit amet consectetur adipisicing elit. Iusto!"),
    ),
    (
        "html",
        42,
        "lorem9|bem|c",
        Some("Lorem ipsum dolor sit amet consectetur adipisicing elit. Iusto!"),
    ),
    (
        "html",
        42,
        "lorem30",
        Some(
            "Lorem ipsum dolor sit amet consectetur adipisicing elit. Iusto accusantium eveniet libero non commodi corrupti doloremque maiores, quam aperiam magni aut dignissimos similique esse dolor voluptates! Beatae nihil repellendus reiciendis?",
        ),
    ),
    (
        "html",
        42,
        "lorem30|bem|c",
        Some(
            "Lorem ipsum dolor sit amet consectetur adipisicing elit. Iusto accusantium eveniet libero non commodi corrupti doloremque maiores, quam aperiam magni aut dignissimos similique esse dolor voluptates! Beatae nihil repellendus reiciendis?",
        ),
    ),
    (
        "html",
        42,
        "lorem100",
        Some(
            "Lorem ipsum dolor sit amet consectetur adipisicing elit. Iusto accusantium eveniet libero non commodi corrupti doloremque maiores, quam aperiam magni aut dignissimos similique esse dolor voluptates! Beatae nihil repellendus reiciendis officiis eveniet delectus magnam quod earum unde voluptatem cupiditate ut rerum maxime libero quisquam animi tempore at magni fugiat, sapiente natus, veritatis laborum ratione? Quia suscipit provident tempore porro vel est fugiat aliquam ut aliquid non ipsam sed distinctio quibusdam, minima modi eaque cupiditate optio inventore recusandae aspernatur, neque odit! Aliquam nobis consectetur repellendus porro, iste vitae sapiente assumenda minima quod, architecto cumque provident totam tempora aspernatur perspiciatis corporis debitis.",
        ),
    ),
    (
        "html",
        42,
        "lorem100|bem|c",
        Some(
            "Lorem ipsum dolor sit amet consectetur adipisicing elit. Iusto accusantium eveniet libero non commodi corrupti doloremque maiores, quam aperiam magni aut dignissimos similique esse dolor voluptates! Beatae nihil repellendus reiciendis officiis eveniet delectus magnam quod earum unde voluptatem cupiditate ut rerum maxime libero quisquam animi tempore at magni fugiat, sapiente natus, veritatis laborum ratione? Quia suscipit provident tempore porro vel est fugiat aliquam ut aliquid non ipsam sed distinctio quibusdam, minima modi eaque cupiditate optio inventore recusandae aspernatur, neque odit! Aliquam nobis consectetur repellendus porro, iste vitae sapiente assumenda minima quod, architecto cumque provident totam tempora aspernatur perspiciatis corporis debitis.",
        ),
    ),
    (
        "html",
        42,
        "lorem5-20",
        Some("Lorem ipsum dolor sit amet consectetur adipisicing elit."),
    ),
    (
        "html",
        42,
        "lorem5-20|bem|c",
        Some("Lorem ipsum dolor sit amet consectetur adipisicing elit."),
    ),
    (
        "html",
        42,
        "lorem20-5",
        Some(
            "Lorem ipsum dolor sit amet consectetur adipisicing elit. Iusto accusantium eveniet libero non commodi corrupti doloremque maiores quam aperiam magni?",
        ),
    ),
    (
        "html",
        42,
        "lorem20-5|bem|c",
        Some(
            "Lorem ipsum dolor sit amet consectetur adipisicing elit. Iusto accusantium eveniet libero non commodi corrupti doloremque maiores quam aperiam magni?",
        ),
    ),
    ("html", 42, "lorem3-", Some("Lorem, ipsum dolor.")),
    ("html", 42, "lorem3-|bem|c", Some("Lorem, ipsum dolor.")),
    (
        "html",
        42,
        "loremru12",
        Some(
            "Далеко-далеко за словесными горами в стране гласных и согласных живут рыбные тексты. Текстами, знаках?",
        ),
    ),
    (
        "html",
        42,
        "loremru12|bem|c",
        Some(
            "Далеко-далеко за словесными горами в стране гласных и согласных живут рыбные тексты. Текстами, знаках?",
        ),
    ),
    (
        "html",
        42,
        "loremsp40",
        Some(
            "Mujer uno dolor más de poder mismo si. Justo accesorio paño libertario no comodidad corrupto paloma mayores, manantial síndrome magnífico auto dignísimo propósito yugo vietnamita fotografía! Corchea nihilista, repelente reincidente oficinista paño, deletéreo magma patrón angustia falda voluntad, encefalograma o?",
        ),
    ),
    (
        "html",
        42,
        "loremsp40|bem|c",
        Some(
            "Mujer uno dolor más de poder mismo si. Justo accesorio paño libertario no comodidad corrupto paloma mayores, manantial síndrome magnífico auto dignísimo propósito yugo vietnamita fotografía! Corchea nihilista, repelente reincidente oficinista paño, deletéreo magma patrón angustia falda voluntad, encefalograma o?",
        ),
    ),
    (
        "html",
        42,
        "loremfr12",
        Some(
            "Lorem ipsum dolor sit amet consectetur adipisicing elit. Iusto accusantium eveniet libero.",
        ),
    ),
    (
        "html",
        42,
        "loremfr12|bem|c",
        Some(
            "Lorem ipsum dolor sit amet consectetur adipisicing elit. Iusto accusantium eveniet libero.",
        ),
    ),
    (
        "html",
        42,
        "LOREMRU12",
        Some(
            "Lorem ipsum dolor sit amet consectetur adipisicing elit. Iusto accusantium eveniet libero.",
        ),
    ),
    (
        "html",
        42,
        "LOREMRU12|bem|c",
        Some(
            "Lorem ipsum dolor sit amet consectetur adipisicing elit. Iusto accusantium eveniet libero.",
        ),
    ),
    ("html", 42, "loremru1", Some("Далеко-далеко.")),
    ("html", 42, "loremru1|bem|c", Some("Далеко-далеко.")),
    ("html", 42, "loremsp1", Some("Mujer.")),
    ("html", 42, "loremsp1|bem|c", Some("Mujer.")),
    (
        "html",
        42,
        "lorem0008",
        Some("Lorem ipsum dolor sit amet consectetur adipisicing elit."),
    ),
    (
        "html",
        42,
        "lorem0008|bem|c",
        Some("Lorem ipsum dolor sit amet consectetur adipisicing elit."),
    ),
    (
        "html",
        42,
        "lorem-8",
        Some(
            "Lorem ipsum dolor sit amet consectetur adipisicing elit. Iusto accusantium eveniet libero non commodi corrupti doloremque maiores, quam aperiam magni aut dignissimos similique esse dolor voluptates! Beatae nihil repellendus reiciendis?",
        ),
    ),
    (
        "html",
        42,
        "lorem-8|bem|c",
        Some(
            "Lorem ipsum dolor sit amet consectetur adipisicing elit. Iusto accusantium eveniet libero non commodi corrupti doloremque maiores, quam aperiam magni aut dignissimos similique esse dolor voluptates! Beatae nihil repellendus reiciendis?",
        ),
    ),
    (
        "html",
        42,
        "lorem-0",
        Some(
            "Lorem ipsum dolor sit amet consectetur adipisicing elit. Iusto accusantium eveniet libero non commodi corrupti doloremque maiores, quam aperiam magni aut dignissimos similique esse dolor voluptates! Beatae nihil repellendus reiciendis?",
        ),
    ),
    (
        "html",
        42,
        "lorem-0|bem|c",
        Some(
            "Lorem ipsum dolor sit amet consectetur adipisicing elit. Iusto accusantium eveniet libero non commodi corrupti doloremque maiores, quam aperiam magni aut dignissimos similique esse dolor voluptates! Beatae nihil repellendus reiciendis?",
        ),
    ),
    (
        "html",
        42,
        "lorem*3",
        Some(
            "Lorem ipsum dolor sit amet consectetur adipisicing elit. Iusto accusantium eveniet libero non commodi corrupti doloremque maiores, quam aperiam magni aut dignissimos similique esse dolor voluptates! Beatae nihil repellendus reiciendis?\nQuod earum unde voluptatem cupiditate ut rerum maxime, libero quisquam animi tempore at magni fugiat sapiente natus veritatis laborum ratione necessitatibus optio eos cumque asperiores? Vel est fugiat aliquam ut?\nSed distinctio quibusdam minima modi. Inventore cupiditate recusandae aspernatur neque odit ducimus, quae facere quo, ad aliquam nobis consectetur repellendus porro iste vitae sapiente assumenda minima quod architecto cumque provident?",
        ),
    ),
    (
        "html",
        42,
        "lorem*3|bem|c",
        Some(
            "Lorem ipsum dolor sit amet consectetur adipisicing elit. Iusto accusantium eveniet libero non commodi corrupti doloremque maiores, quam aperiam magni aut dignissimos similique esse dolor voluptates! Beatae nihil repellendus reiciendis?\nQuod earum unde voluptatem cupiditate ut rerum maxime, libero quisquam animi tempore at magni fugiat sapiente natus veritatis laborum ratione necessitatibus optio eos cumque asperiores? Vel est fugiat aliquam ut?\nSed distinctio quibusdam minima modi. Inventore cupiditate recusandae aspernatur neque odit ducimus, quae facere quo, ad aliquam nobis consectetur repellendus porro iste vitae sapiente assumenda minima quod architecto cumque provident?",
        ),
    ),
    (
        "html",
        42,
        "lorem5*3",
        Some(
            "Lorem ipsum dolor sit amet.\nAccusantium eveniet libero non commodi.\nAperiam magni aut dignissimos similique.",
        ),
    ),
    (
        "html",
        42,
        "lorem5*3|bem|c",
        Some(
            "Lorem ipsum dolor sit amet.\nAccusantium eveniet libero non commodi.\nAperiam magni aut dignissimos similique.",
        ),
    ),
    (
        "html",
        42,
        "p>lorem5*3",
        Some(
            "<p><span>Lorem ipsum dolor sit amet.</span><span>Accusantium eveniet libero non commodi.</span><span>Aperiam magni aut dignissimos similique.</span></p>",
        ),
    ),
    (
        "html",
        42,
        "p>lorem5*3|bem|c",
        Some(
            "<p><span>Lorem ipsum dolor sit amet.</span><span>Accusantium eveniet libero non commodi.</span><span>Aperiam magni aut dignissimos similique.</span></p>",
        ),
    ),
    (
        "html",
        42,
        "p*3>lorem12",
        Some(
            "<p>Lorem ipsum dolor sit amet consectetur adipisicing elit. Iusto accusantium eveniet libero.</p>\n<p>Maiores quam aperiam magni aut dignissimos similique esse dolor, voluptates ipsum voluptatibus?</p>\n<p>Officiis eveniet delectus magnam quod earum unde voluptatem cupiditate ut rerum maxime?</p>",
        ),
    ),
    (
        "html",
        42,
        "p*3>lorem12|bem|c",
        Some(
            "<p>Lorem ipsum dolor sit amet consectetur adipisicing elit. Iusto accusantium eveniet libero.</p>\n<p>Maiores quam aperiam magni aut dignissimos similique esse dolor, voluptates ipsum voluptatibus?</p>\n<p>Officiis eveniet delectus magnam quod earum unde voluptatem cupiditate ut rerum maxime?</p>",
        ),
    ),
    (
        "html",
        42,
        "ul>lorem8*2",
        Some(
            "<ul>\n\t<li>Lorem ipsum dolor sit amet consectetur adipisicing elit.</li>\n\t<li>Accusantium eveniet libero non commodi corrupti, doloremque maiores.</li>\n</ul>",
        ),
    ),
    (
        "html",
        42,
        "ul>lorem8*2|bem|c",
        Some(
            "<ul>\n\t<li>Lorem ipsum dolor sit amet consectetur adipisicing elit.</li>\n\t<li>Accusantium eveniet libero non commodi corrupti, doloremque maiores.</li>\n</ul>",
        ),
    ),
    (
        "html",
        42,
        "ul*2>li>lorem10",
        Some(
            "<ul>\n\t<li>Lorem ipsum dolor sit amet consectetur adipisicing elit. Iusto, accusantium?</li>\n</ul>\n<ul>\n\t<li>Doloremque maiores, quam aperiam magni aut dignissimos similique esse dolor.</li>\n</ul>",
        ),
    ),
    (
        "html",
        42,
        "ul*2>li>lorem10|bem|c",
        Some(
            "<ul>\n\t<li>Lorem ipsum dolor sit amet consectetur adipisicing elit. Iusto, accusantium?</li>\n</ul>\n<ul>\n\t<li>Doloremque maiores, quam aperiam magni aut dignissimos similique esse dolor.</li>\n</ul>",
        ),
    ),
    (
        "html",
        42,
        "(h2{title}+p>lorem12)*2",
        Some(
            "<h2>title</h2>\n<p>Lorem ipsum dolor sit amet consectetur adipisicing elit. Iusto accusantium eveniet libero.</p>\n<h2>title</h2>\n<p>Maiores quam aperiam magni aut dignissimos similique esse dolor, voluptates ipsum voluptatibus?</p>",
        ),
    ),
    (
        "html",
        42,
        "(h2{title}+p>lorem12)*2|bem|c",
        Some(
            "<h2>title</h2>\n<p>Lorem ipsum dolor sit amet consectetur adipisicing elit. Iusto accusantium eveniet libero.</p>\n<h2>title</h2>\n<p>Maiores quam aperiam magni aut dignissimos similique esse dolor, voluptates ipsum voluptatibus?</p>",
        ),
    ),
    (
        "html",
        42,
        "div>lorem5>span{end}",
        Some("<div>Lorem ipsum dolor sit amet.<span>end</span></div>"),
    ),
    (
        "html",
        42,
        "div>lorem5>span{end}|bem|c",
        Some("<div>Lorem ipsum dolor sit amet.<span>end</span></div>"),
    ),
    (
        "html",
        42,
        "p>lorem5.class#id[title=ignored]",
        Some("<p>Lorem ipsum dolor sit amet.</p>"),
    ),
    (
        "html",
        42,
        "p>lorem5.class#id[title=ignored]|bem|c",
        Some("<p>Lorem ipsum dolor sit amet.</p>"),
    ),
    (
        "html",
        42,
        "div.block>p.-element>lorem12",
        Some(
            "<div class=\"block\">\n\t<p class=\"-element\">Lorem ipsum dolor sit amet consectetur adipisicing elit. Iusto accusantium eveniet libero.</p>\n</div>",
        ),
    ),
    (
        "html",
        42,
        "div.block>p.-element>lorem12|bem|c",
        Some(
            "<div class=\"block\">\n\t<p class=\"block__element\">Lorem ipsum dolor sit amet consectetur adipisicing elit. Iusto accusantium eveniet libero.</p>\n\t<!-- /.block__element -->\n</div>\n<!-- /.block -->",
        ),
    ),
    (
        "html",
        42,
        "lorem5{ignored}",
        Some("Lorem ipsum dolor sit amet."),
    ),
    (
        "html",
        42,
        "lorem5{ignored}|bem|c",
        Some("Lorem ipsum dolor sit amet."),
    ),
    (
        "html",
        42,
        "loremx5+span",
        Some("Lorem ipsum dolor sit amet.<span>${0}</span>"),
    ),
    (
        "html",
        42,
        "loremx5+span|bem|c",
        Some("Lorem ipsum dolor sit amet.<span>${0}</span>"),
    ),
    ("html", 42, "lorem5x", Some("<lorem5x>${0}</lorem5x>")),
    ("html", 42, "lorem5x|bem|c", Some("<lorem5x>${0}</lorem5x>")),
    ("html", 42, "lorem1.5", Some("Lorem.")),
    ("html", 42, "lorem1.5|bem|c", Some("Lorem.")),
    (
        "html",
        42,
        "lorem2-4-6",
        Some("<lorem2-4-6>${0}</lorem2-4-6>"),
    ),
    (
        "html",
        42,
        "lorem2-4-6|bem|c",
        Some("<lorem2-4-6>${0}</lorem2-4-6>"),
    ),
    (
        "html",
        42,
        "p>loremru12*2",
        Some(
            "<p><span>Далеко-далеко за словесными горами в стране гласных и согласных живут рыбные тексты. Текстами, знаках?</span><span>Повстречался пустился скатился, коварный деревни обеспечивает которой проектах использовало языком взгляд всемогущая.</span></p>",
        ),
    ),
    (
        "html",
        42,
        "p>loremru12*2|bem|c",
        Some(
            "<p><span>Далеко-далеко за словесными горами в стране гласных и согласных живут рыбные тексты. Текстами, знаках?</span><span>Повстречался пустился скатился, коварный деревни обеспечивает которой проектах использовало языком взгляд всемогущая.</span></p>",
        ),
    ),
    (
        "html",
        42,
        "table>loremsp9*2",
        Some(
            "<table>\n\t<tr>Mujer uno dolor más de poder mismo si. Justo!</tr>\n\t<tr>No comodidad corrupto paloma mayores manantial síndrome magnífico auto.</tr>\n</table>",
        ),
    ),
    (
        "html",
        42,
        "table>loremsp9*2|bem|c",
        Some(
            "<table>\n\t<tr>Mujer uno dolor más de poder mismo si. Justo!</tr>\n\t<tr>No comodidad corrupto paloma mayores manantial síndrome magnífico auto.</tr>\n</table>",
        ),
    ),
    (
        "html",
        987654321,
        "lorem",
        Some(
            "Lorem ipsum dolor sit, amet consectetur adipisicing elit. Deserunt voluptas quos nam, nisi nesciunt mollitia veritatis odit hic dolores, nemo ex. Quod molestias ex rerum, natus amet voluptatum fugiat mollitia.",
        ),
    ),
    (
        "html",
        987654321,
        "lorem|bem|c",
        Some(
            "Lorem ipsum dolor sit, amet consectetur adipisicing elit. Deserunt voluptas quos nam, nisi nesciunt mollitia veritatis odit hic dolores, nemo ex. Quod molestias ex rerum, natus amet voluptatum fugiat mollitia.",
        ),
    ),
    ("html", 987654321, "lorem0", Some("Lorem.")),
    ("html", 987654321, "lorem0|bem|c", Some("Lorem.")),
    ("html", 987654321, "lorem1", Some("Lorem.")),
    ("html", 987654321, "lorem1|bem|c", Some("Lorem.")),
    ("html", 987654321, "lorem2", Some("Lorem, ipsum.")),
    ("html", 987654321, "lorem2|bem|c", Some("Lorem, ipsum.")),
    (
        "html",
        987654321,
        "lorem8",
        Some("Lorem ipsum dolor sit, amet consectetur adipisicing elit."),
    ),
    (
        "html",
        987654321,
        "lorem8|bem|c",
        Some("Lorem ipsum dolor sit, amet consectetur adipisicing elit."),
    ),
    (
        "html",
        987654321,
        "lorem9",
        Some("Lorem ipsum dolor sit, amet consectetur adipisicing elit. Deserunt!"),
    ),
    (
        "html",
        987654321,
        "lorem9|bem|c",
        Some("Lorem ipsum dolor sit, amet consectetur adipisicing elit. Deserunt!"),
    ),
    (
        "html",
        987654321,
        "lorem30",
        Some(
            "Lorem ipsum dolor sit, amet consectetur adipisicing elit. Deserunt voluptas quos nam, nisi nesciunt mollitia veritatis odit hic dolores, nemo ex. Quod molestias ex rerum, natus amet voluptatum fugiat mollitia.",
        ),
    ),
    (
        "html",
        987654321,
        "lorem30|bem|c",
        Some(
            "Lorem ipsum dolor sit, amet consectetur adipisicing elit. Deserunt voluptas quos nam, nisi nesciunt mollitia veritatis odit hic dolores, nemo ex. Quod molestias ex rerum, natus amet voluptatum fugiat mollitia.",
        ),
    ),
    (
        "html",
        987654321,
        "lorem100",
        Some(
            "Lorem ipsum dolor sit, amet consectetur adipisicing elit. Deserunt voluptas quos nam, nisi nesciunt mollitia veritatis odit hic dolores, nemo ex. Quod molestias ex rerum natus amet voluptatum fugiat mollitia nemo delectus illum harum aperiam maiores praesentium quasi, vero hic temporibus doloremque porro beatae? Obcaecati aperiam, a eum sapiente fuga, maiores inventore dolor accusamus quis reiciendis impedit deleniti, doloribus voluptatibus soluta id vero velit dolorem ipsa incidunt. Sit beatae facilis, totam sint quod quaerat sunt molestiae culpa optio quam aperiam libero delectus voluptates incidunt ea! Doloribus assumenda unde tempore tenetur animi? Aut quod molestiae suscipit esse adipisci. Perferendis, incidunt odit.",
        ),
    ),
    (
        "html",
        987654321,
        "lorem100|bem|c",
        Some(
            "Lorem ipsum dolor sit, amet consectetur adipisicing elit. Deserunt voluptas quos nam, nisi nesciunt mollitia veritatis odit hic dolores, nemo ex. Quod molestias ex rerum natus amet voluptatum fugiat mollitia nemo delectus illum harum aperiam maiores praesentium quasi, vero hic temporibus doloremque porro beatae? Obcaecati aperiam, a eum sapiente fuga, maiores inventore dolor accusamus quis reiciendis impedit deleniti, doloribus voluptatibus soluta id vero velit dolorem ipsa incidunt. Sit beatae facilis, totam sint quod quaerat sunt molestiae culpa optio quam aperiam libero delectus voluptates incidunt ea! Doloribus assumenda unde tempore tenetur animi? Aut quod molestiae suscipit esse adipisci. Perferendis, incidunt odit.",
        ),
    ),
    (
        "html",
        987654321,
        "lorem5-20",
        Some("Lorem ipsum dolor sit amet."),
    ),
    (
        "html",
        987654321,
        "lorem5-20|bem|c",
        Some("Lorem ipsum dolor sit amet."),
    ),
    (
        "html",
        987654321,
        "lorem20-5",
        Some(
            "Lorem ipsum dolor sit, amet consectetur adipisicing elit. Deserunt voluptas quos nam nisi nesciunt mollitia veritatis odit hic dolores nemo.",
        ),
    ),
    (
        "html",
        987654321,
        "lorem20-5|bem|c",
        Some(
            "Lorem ipsum dolor sit, amet consectetur adipisicing elit. Deserunt voluptas quos nam nisi nesciunt mollitia veritatis odit hic dolores nemo.",
        ),
    ),
    ("html", 987654321, "lorem3-", Some("Lorem, ipsum dolor.")),
    (
        "html",
        987654321,
        "lorem3-|bem|c",
        Some("Lorem, ipsum dolor."),
    ),
    (
        "html",
        987654321,
        "loremru12",
        Some(
            "Далеко-далеко за словесными горами в стране, гласных и согласных живут рыбные тексты. Свой, злых.",
        ),
    ),
    (
        "html",
        987654321,
        "loremru12|bem|c",
        Some(
            "Далеко-далеко за словесными горами в стране, гласных и согласных живут рыбные тексты. Свой, злых.",
        ),
    ),
    (
        "html",
        987654321,
        "loremsp40",
        Some(
            "Mujer uno dolor más, de poder mismo si. Desierto voluble pan namibia, león nascimiento computador verdades odio alma dolores, remo ex. Patrón molestias ex renuncia natividad entero torre refugiado computador remo deletéreo marejada olvido síndrome, mayores, presentación casi, alveolar alma?",
        ),
    ),
    (
        "html",
        987654321,
        "loremsp40|bem|c",
        Some(
            "Mujer uno dolor más, de poder mismo si. Desierto voluble pan namibia, león nascimiento computador verdades odio alma dolores, remo ex. Patrón molestias ex renuncia natividad entero torre refugiado computador remo deletéreo marejada olvido síndrome, mayores, presentación casi, alveolar alma?",
        ),
    ),
    (
        "html",
        987654321,
        "loremfr12",
        Some(
            "Lorem ipsum dolor sit, amet consectetur adipisicing elit. Deserunt voluptas quos nam!",
        ),
    ),
    (
        "html",
        987654321,
        "loremfr12|bem|c",
        Some(
            "Lorem ipsum dolor sit, amet consectetur adipisicing elit. Deserunt voluptas quos nam!",
        ),
    ),
    (
        "html",
        987654321,
        "LOREMRU12",
        Some(
            "Lorem ipsum dolor sit, amet consectetur adipisicing elit. Deserunt voluptas quos nam!",
        ),
    ),
    (
        "html",
        987654321,
        "LOREMRU12|bem|c",
        Some(
            "Lorem ipsum dolor sit, amet consectetur adipisicing elit. Deserunt voluptas quos nam!",
        ),
    ),
    ("html", 987654321, "loremru1", Some("Далеко-далеко.")),
    ("html", 987654321, "loremru1|bem|c", Some("Далеко-далеко.")),
    ("html", 987654321, "loremsp1", Some("Mujer.")),
    ("html", 987654321, "loremsp1|bem|c", Some("Mujer.")),
    (
        "html",
        987654321,
        "lorem0008",
        Some("Lorem ipsum dolor sit, amet consectetur adipisicing elit."),
    ),
    (
        "html",
        987654321,
        "lorem0008|bem|c",
        Some("Lorem ipsum dolor sit, amet consectetur adipisicing elit."),
    ),
    (
        "html",
        987654321,
        "lorem-8",
        Some(
            "Lorem ipsum dolor sit, amet consectetur adipisicing elit. Deserunt voluptas quos nam, nisi nesciunt mollitia veritatis odit hic dolores, nemo ex. Quod molestias ex rerum, natus amet voluptatum fugiat mollitia.",
        ),
    ),
    (
        "html",
        987654321,
        "lorem-8|bem|c",
        Some(
            "Lorem ipsum dolor sit, amet consectetur adipisicing elit. Deserunt voluptas quos nam, nisi nesciunt mollitia veritatis odit hic dolores, nemo ex. Quod molestias ex rerum, natus amet voluptatum fugiat mollitia.",
        ),
    ),
    (
        "html",
        987654321,
        "lorem-0",
        Some(
            "Lorem ipsum dolor sit, amet consectetur adipisicing elit. Deserunt voluptas quos nam, nisi nesciunt mollitia veritatis odit hic dolores, nemo ex. Quod molestias ex rerum, natus amet voluptatum fugiat mollitia.",
        ),
    ),
    (
        "html",
        987654321,
        "lorem-0|bem|c",
        Some(
            "Lorem ipsum dolor sit, amet consectetur adipisicing elit. Deserunt voluptas quos nam, nisi nesciunt mollitia veritatis odit hic dolores, nemo ex. Quod molestias ex rerum, natus amet voluptatum fugiat mollitia.",
        ),
    ),
    (
        "html",
        987654321,
        "lorem*3",
        Some(
            "Lorem ipsum dolor sit, amet consectetur adipisicing elit. Deserunt voluptas quos nam, nisi nesciunt mollitia veritatis odit hic dolores, nemo ex. Quod molestias ex rerum, natus amet voluptatum fugiat mollitia.\nMaiores praesentium quasi vero hic temporibus doloremque, porro, beatae velit at iusto nemo obcaecati aperiam a eum sapiente fuga inventore dolor accusamus quis reiciendis impedit deleniti! Vero velit dolorem ipsa.\nSed distinctio omnis sit beatae facilis totam sint quod quaerat sunt, molestiae culpa optio quam aperiam libero delectus voluptates! Autem facilis doloribus assumenda unde. Porro quisquam facilis aut quod molestiae.",
        ),
    ),
    (
        "html",
        987654321,
        "lorem*3|bem|c",
        Some(
            "Lorem ipsum dolor sit, amet consectetur adipisicing elit. Deserunt voluptas quos nam, nisi nesciunt mollitia veritatis odit hic dolores, nemo ex. Quod molestias ex rerum, natus amet voluptatum fugiat mollitia.\nMaiores praesentium quasi vero hic temporibus doloremque, porro, beatae velit at iusto nemo obcaecati aperiam a eum sapiente fuga inventore dolor accusamus quis reiciendis impedit deleniti! Vero velit dolorem ipsa.\nSed distinctio omnis sit beatae facilis totam sint quod quaerat sunt, molestiae culpa optio quam aperiam libero delectus voluptates! Autem facilis doloribus assumenda unde. Porro quisquam facilis aut quod molestiae.",
        ),
    ),
    (
        "html",
        987654321,
        "lorem5*3",
        Some(
            "Lorem ipsum dolor sit amet.\nDeserunt voluptas quos nam nisi.\nHic dolores nemo ex cupiditate!",
        ),
    ),
    (
        "html",
        987654321,
        "lorem5*3|bem|c",
        Some(
            "Lorem ipsum dolor sit amet.\nDeserunt voluptas quos nam nisi.\nHic dolores nemo ex cupiditate!",
        ),
    ),
    (
        "html",
        987654321,
        "p>lorem5*3",
        Some(
            "<p><span>Lorem ipsum dolor sit amet.</span><span>Deserunt voluptas quos nam nisi.</span><span>Hic dolores nemo ex cupiditate!</span></p>",
        ),
    ),
    (
        "html",
        987654321,
        "p>lorem5*3|bem|c",
        Some(
            "<p><span>Lorem ipsum dolor sit amet.</span><span>Deserunt voluptas quos nam nisi.</span><span>Hic dolores nemo ex cupiditate!</span></p>",
        ),
    ),
    (
        "html",
        987654321,
        "p*3>lorem12",
        Some(
            "<p>Lorem ipsum dolor sit, amet consectetur adipisicing elit. Deserunt voluptas quos nam!</p>\n<p>Odit hic dolores nemo ex cupiditate, praesentium esse quia. Rerum, natus amet.</p>\n<p>Harum aperiam maiores, praesentium quasi vero hic temporibus doloremque porro beatae velit.</p>",
        ),
    ),
    (
        "html",
        987654321,
        "p*3>lorem12|bem|c",
        Some(
            "<p>Lorem ipsum dolor sit, amet consectetur adipisicing elit. Deserunt voluptas quos nam!</p>\n<p>Odit hic dolores nemo ex cupiditate, praesentium esse quia. Rerum, natus amet.</p>\n<p>Harum aperiam maiores, praesentium quasi vero hic temporibus doloremque porro beatae velit.</p>",
        ),
    ),
    (
        "html",
        987654321,
        "ul>lorem8*2",
        Some(
            "<ul>\n\t<li>Lorem ipsum dolor sit, amet consectetur adipisicing elit.</li>\n\t<li>Voluptas quos nam nisi, nesciunt mollitia veritatis odit.</li>\n</ul>",
        ),
    ),
    (
        "html",
        987654321,
        "ul>lorem8*2|bem|c",
        Some(
            "<ul>\n\t<li>Lorem ipsum dolor sit, amet consectetur adipisicing elit.</li>\n\t<li>Voluptas quos nam nisi, nesciunt mollitia veritatis odit.</li>\n</ul>",
        ),
    ),
    (
        "html",
        987654321,
        "ul*2>li>lorem10",
        Some(
            "<ul>\n\t<li>Lorem ipsum dolor sit, amet consectetur adipisicing elit. Deserunt, voluptas.</li>\n</ul>\n<ul>\n\t<li>Hic dolores nemo ex cupiditate! Quia at quod molestias ex?</li>\n</ul>",
        ),
    ),
    (
        "html",
        987654321,
        "ul*2>li>lorem10|bem|c",
        Some(
            "<ul>\n\t<li>Lorem ipsum dolor sit, amet consectetur adipisicing elit. Deserunt, voluptas.</li>\n</ul>\n<ul>\n\t<li>Hic dolores nemo ex cupiditate! Quia at quod molestias ex?</li>\n</ul>",
        ),
    ),
    (
        "html",
        987654321,
        "(h2{title}+p>lorem12)*2",
        Some(
            "<h2>title</h2>\n<p>Lorem ipsum dolor sit, amet consectetur adipisicing elit. Deserunt voluptas quos nam!</p>\n<h2>title</h2>\n<p>Odit hic dolores nemo ex cupiditate, praesentium esse quia. Rerum, natus amet.</p>",
        ),
    ),
    (
        "html",
        987654321,
        "(h2{title}+p>lorem12)*2|bem|c",
        Some(
            "<h2>title</h2>\n<p>Lorem ipsum dolor sit, amet consectetur adipisicing elit. Deserunt voluptas quos nam!</p>\n<h2>title</h2>\n<p>Odit hic dolores nemo ex cupiditate, praesentium esse quia. Rerum, natus amet.</p>",
        ),
    ),
    (
        "html",
        987654321,
        "div>lorem5>span{end}",
        Some("<div>Lorem ipsum dolor sit amet.<span>end</span></div>"),
    ),
    (
        "html",
        987654321,
        "div>lorem5>span{end}|bem|c",
        Some("<div>Lorem ipsum dolor sit amet.<span>end</span></div>"),
    ),
    (
        "html",
        987654321,
        "p>lorem5.class#id[title=ignored]",
        Some("<p>Lorem ipsum dolor sit amet.</p>"),
    ),
    (
        "html",
        987654321,
        "p>lorem5.class#id[title=ignored]|bem|c",
        Some("<p>Lorem ipsum dolor sit amet.</p>"),
    ),
    (
        "html",
        987654321,
        "div.block>p.-element>lorem12",
        Some(
            "<div class=\"block\">\n\t<p class=\"-element\">Lorem ipsum dolor sit, amet consectetur adipisicing elit. Deserunt voluptas quos nam!</p>\n</div>",
        ),
    ),
    (
        "html",
        987654321,
        "div.block>p.-element>lorem12|bem|c",
        Some(
            "<div class=\"block\">\n\t<p class=\"block__element\">Lorem ipsum dolor sit, amet consectetur adipisicing elit. Deserunt voluptas quos nam!</p>\n\t<!-- /.block__element -->\n</div>\n<!-- /.block -->",
        ),
    ),
    (
        "html",
        987654321,
        "lorem5{ignored}",
        Some("Lorem ipsum dolor sit amet."),
    ),
    (
        "html",
        987654321,
        "lorem5{ignored}|bem|c",
        Some("Lorem ipsum dolor sit amet."),
    ),
    (
        "html",
        987654321,
        "loremx5+span",
        Some("Lorem ipsum dolor sit amet.<span>${0}</span>"),
    ),
    (
        "html",
        987654321,
        "loremx5+span|bem|c",
        Some("Lorem ipsum dolor sit amet.<span>${0}</span>"),
    ),
    (
        "html",
        987654321,
        "lorem5x",
        Some("<lorem5x>${0}</lorem5x>"),
    ),
    (
        "html",
        987654321,
        "lorem5x|bem|c",
        Some("<lorem5x>${0}</lorem5x>"),
    ),
    ("html", 987654321, "lorem1.5", Some("Lorem.")),
    ("html", 987654321, "lorem1.5|bem|c", Some("Lorem.")),
    (
        "html",
        987654321,
        "lorem2-4-6",
        Some("<lorem2-4-6>${0}</lorem2-4-6>"),
    ),
    (
        "html",
        987654321,
        "lorem2-4-6|bem|c",
        Some("<lorem2-4-6>${0}</lorem2-4-6>"),
    ),
    (
        "html",
        987654321,
        "p>loremru12*2",
        Some(
            "<p><span>Далеко-далеко за словесными горами в стране, гласных и согласных живут рыбные тексты. Свой, злых.</span><span>Рукопись назад меня однажды она! Города это вскоре заголовок однажды все рыбными.</span></p>",
        ),
    ),
    (
        "html",
        987654321,
        "p>loremru12*2|bem|c",
        Some(
            "<p><span>Далеко-далеко за словесными горами в стране, гласных и согласных живут рыбные тексты. Свой, злых.</span><span>Рукопись назад меня однажды она! Города это вскоре заголовок однажды все рыбными.</span></p>",
        ),
    ),
    (
        "html",
        987654321,
        "table>loremsp9*2",
        Some(
            "<table>\n\t<tr>Mujer uno dolor más, de poder mismo si. Desierto!</tr>\n\t<tr>León, nascimiento computador. Remo ex encefalograma presentación yugo resiliencia.</tr>\n</table>",
        ),
    ),
    (
        "html",
        987654321,
        "table>loremsp9*2|bem|c",
        Some(
            "<table>\n\t<tr>Mujer uno dolor más, de poder mismo si. Desierto!</tr>\n\t<tr>León, nascimiento computador. Remo ex encefalograma presentación yugo resiliencia.</tr>\n</table>",
        ),
    ),
    (
        "jsx",
        1,
        "lorem",
        Some(
            "Lorem ipsum dolor sit amet consectetur adipisicing elit. Accusamus eum, sint id, dolores laborum quam ex amet molestias recusandae sequi porro aperiam rem consequuntur? Tempore accusamus voluptatem unde voluptatibus nemo?",
        ),
    ),
    (
        "jsx",
        1,
        "lorem|bem|c",
        Some(
            "Lorem ipsum dolor sit amet consectetur adipisicing elit. Accusamus eum, sint id, dolores laborum quam ex amet molestias recusandae sequi porro aperiam rem consequuntur? Tempore accusamus voluptatem unde voluptatibus nemo?",
        ),
    ),
    ("jsx", 1, "lorem0", Some("Lorem.")),
    ("jsx", 1, "lorem0|bem|c", Some("Lorem.")),
    ("jsx", 1, "lorem1", Some("Lorem.")),
    ("jsx", 1, "lorem1|bem|c", Some("Lorem.")),
    ("jsx", 1, "lorem2", Some("Lorem, ipsum.")),
    ("jsx", 1, "lorem2|bem|c", Some("Lorem, ipsum.")),
    (
        "jsx",
        1,
        "lorem8",
        Some("Lorem ipsum dolor sit amet consectetur adipisicing elit."),
    ),
    (
        "jsx",
        1,
        "lorem8|bem|c",
        Some("Lorem ipsum dolor sit amet consectetur adipisicing elit."),
    ),
    (
        "jsx",
        1,
        "lorem9",
        Some("Lorem ipsum dolor sit amet consectetur adipisicing elit. Accusamus?"),
    ),
    (
        "jsx",
        1,
        "lorem9|bem|c",
        Some("Lorem ipsum dolor sit amet consectetur adipisicing elit. Accusamus?"),
    ),
    (
        "jsx",
        1,
        "lorem30",
        Some(
            "Lorem ipsum dolor sit amet consectetur adipisicing elit. Accusamus eum, sint id, dolores laborum quam ex amet molestias recusandae sequi porro aperiam rem consequuntur? Tempore accusamus voluptatem unde voluptatibus nemo?",
        ),
    ),
    (
        "jsx",
        1,
        "lorem30|bem|c",
        Some(
            "Lorem ipsum dolor sit amet consectetur adipisicing elit. Accusamus eum, sint id, dolores laborum quam ex amet molestias recusandae sequi porro aperiam rem consequuntur? Tempore accusamus voluptatem unde voluptatibus nemo?",
        ),
    ),
    (
        "jsx",
        1,
        "lorem100",
        Some(
            "Lorem ipsum dolor sit amet consectetur adipisicing elit. Accusamus eum, sint id, dolores laborum quam ex amet molestias recusandae sequi porro aperiam rem consequuntur? Tempore accusamus voluptatem unde voluptatibus nemo mollitia ipsum necessitatibus porro. Maxime, commodi ut alias, deleniti nihil sint consequuntur expedita minima vero, molestias asperiores necessitatibus laborum! Adipisci, tempore sit. Perspiciatis sunt corrupti fugiat ipsa voluptate laboriosam dolore eius impedit soluta facilis repellat ad quas quia ducimus eum, amet molestiae similique suscipit id veritatis veniam a aliquam possimus. Maiores recusandae assumenda exercitationem, quidem atque, consectetur nostrum natus sed in repudiandae, illum quia. Ipsa distinctio non nulla repellendus perferendis.",
        ),
    ),
    (
        "jsx",
        1,
        "lorem100|bem|c",
        Some(
            "Lorem ipsum dolor sit amet consectetur adipisicing elit. Accusamus eum, sint id, dolores laborum quam ex amet molestias recusandae sequi porro aperiam rem consequuntur? Tempore accusamus voluptatem unde voluptatibus nemo mollitia ipsum necessitatibus porro. Maxime, commodi ut alias, deleniti nihil sint consequuntur expedita minima vero, molestias asperiores necessitatibus laborum! Adipisci, tempore sit. Perspiciatis sunt corrupti fugiat ipsa voluptate laboriosam dolore eius impedit soluta facilis repellat ad quas quia ducimus eum, amet molestiae similique suscipit id veritatis veniam a aliquam possimus. Maiores recusandae assumenda exercitationem, quidem atque, consectetur nostrum natus sed in repudiandae, illum quia. Ipsa distinctio non nulla repellendus perferendis.",
        ),
    ),
    (
        "jsx",
        1,
        "lorem5-20",
        Some("Lorem ipsum dolor sit amet consectetur adipisicing elit."),
    ),
    (
        "jsx",
        1,
        "lorem5-20|bem|c",
        Some("Lorem ipsum dolor sit amet consectetur adipisicing elit."),
    ),
    (
        "jsx",
        1,
        "lorem20-5",
        Some(
            "Lorem ipsum dolor sit amet consectetur adipisicing elit. Accusamus eum sint id dolores laborum quam ex amet, molestias recusandae sequi!",
        ),
    ),
    (
        "jsx",
        1,
        "lorem20-5|bem|c",
        Some(
            "Lorem ipsum dolor sit amet consectetur adipisicing elit. Accusamus eum sint id dolores laborum quam ex amet, molestias recusandae sequi!",
        ),
    ),
    ("jsx", 1, "lorem3-", Some("Lorem, ipsum dolor.")),
    ("jsx", 1, "lorem3-|bem|c", Some("Lorem, ipsum dolor.")),
    (
        "jsx",
        1,
        "loremru12",
        Some(
            "Далеко-далеко за словесными горами в стране гласных и согласных живут рыбные тексты. Предупредила, языкового?",
        ),
    ),
    (
        "jsx",
        1,
        "loremru12|bem|c",
        Some(
            "Далеко-далеко за словесными горами в стране гласных и согласных живут рыбные тексты. Предупредила, языкового?",
        ),
    ),
    (
        "jsx",
        1,
        "loremsp40",
        Some(
            "Mujer uno dolor más de poder mismo si. Cosas mirar, sintió vos, dolores laboral manantial ex entero molestias recusado séquito ventana síndrome lejos afortunado? Temporal cosas voluntad falda voluminoso remo computador aparte necesariamente ventana. Anemia comodidad o alias deleznable nihilista.",
        ),
    ),
    (
        "jsx",
        1,
        "loremsp40|bem|c",
        Some(
            "Mujer uno dolor más de poder mismo si. Cosas mirar, sintió vos, dolores laboral manantial ex entero molestias recusado séquito ventana síndrome lejos afortunado? Temporal cosas voluntad falda voluminoso remo computador aparte necesariamente ventana. Anemia comodidad o alias deleznable nihilista.",
        ),
    ),
    (
        "jsx",
        1,
        "loremfr12",
        Some("Lorem ipsum dolor sit amet consectetur adipisicing elit. Accusamus eum sint id?"),
    ),
    (
        "jsx",
        1,
        "loremfr12|bem|c",
        Some("Lorem ipsum dolor sit amet consectetur adipisicing elit. Accusamus eum sint id?"),
    ),
    (
        "jsx",
        1,
        "LOREMRU12",
        Some("Lorem ipsum dolor sit amet consectetur adipisicing elit. Accusamus eum sint id?"),
    ),
    (
        "jsx",
        1,
        "LOREMRU12|bem|c",
        Some("Lorem ipsum dolor sit amet consectetur adipisicing elit. Accusamus eum sint id?"),
    ),
    ("jsx", 1, "loremru1", Some("Далеко-далеко.")),
    ("jsx", 1, "loremru1|bem|c", Some("Далеко-далеко.")),
    ("jsx", 1, "loremsp1", Some("Mujer.")),
    ("jsx", 1, "loremsp1|bem|c", Some("Mujer.")),
    (
        "jsx",
        1,
        "lorem0008",
        Some("Lorem ipsum dolor sit amet consectetur adipisicing elit."),
    ),
    (
        "jsx",
        1,
        "lorem0008|bem|c",
        Some("Lorem ipsum dolor sit amet consectetur adipisicing elit."),
    ),
    (
        "jsx",
        1,
        "lorem-8",
        Some(
            "Lorem ipsum dolor sit amet consectetur adipisicing elit. Accusamus eum, sint id, dolores laborum quam ex amet molestias recusandae sequi porro aperiam rem consequuntur? Tempore accusamus voluptatem unde voluptatibus nemo?",
        ),
    ),
    (
        "jsx",
        1,
        "lorem-8|bem|c",
        Some(
            "Lorem ipsum dolor sit amet consectetur adipisicing elit. Accusamus eum, sint id, dolores laborum quam ex amet molestias recusandae sequi porro aperiam rem consequuntur? Tempore accusamus voluptatem unde voluptatibus nemo?",
        ),
    ),
    (
        "jsx",
        1,
        "lorem-0",
        Some(
            "Lorem ipsum dolor sit amet consectetur adipisicing elit. Accusamus eum, sint id, dolores laborum quam ex amet molestias recusandae sequi porro aperiam rem consequuntur? Tempore accusamus voluptatem unde voluptatibus nemo?",
        ),
    ),
    (
        "jsx",
        1,
        "lorem-0|bem|c",
        Some(
            "Lorem ipsum dolor sit amet consectetur adipisicing elit. Accusamus eum, sint id, dolores laborum quam ex amet molestias recusandae sequi porro aperiam rem consequuntur? Tempore accusamus voluptatem unde voluptatibus nemo?",
        ),
    ),
    (
        "jsx",
        1,
        "lorem*3",
        Some(
            "Lorem ipsum dolor sit amet consectetur adipisicing elit. Accusamus eum, sint id, dolores laborum quam ex amet molestias recusandae sequi porro aperiam rem consequuntur? Tempore accusamus voluptatem unde voluptatibus nemo?\nSapiente quis reiciendis maxime commodi ut alias deleniti nihil sint consequuntur expedita minima vero molestias asperiores necessitatibus laborum eos sequi magnam eveniet, unde nobis adipisci tempore, sit laudantium facilis? Corrupti.\nLaboriosam dolore eius impedit soluta facilis repellat ad quas quia ducimus eum. Suscipit id veritatis veniam, a aliquam possimus laborum atque nemo culpa maiores recusandae assumenda exercitationem quidem consectetur nostrum.",
        ),
    ),
    (
        "jsx",
        1,
        "lorem*3|bem|c",
        Some(
            "Lorem ipsum dolor sit amet consectetur adipisicing elit. Accusamus eum, sint id, dolores laborum quam ex amet molestias recusandae sequi porro aperiam rem consequuntur? Tempore accusamus voluptatem unde voluptatibus nemo?\nSapiente quis reiciendis maxime commodi ut alias deleniti nihil sint consequuntur expedita minima vero molestias asperiores necessitatibus laborum eos sequi magnam eveniet, unde nobis adipisci tempore, sit laudantium facilis? Corrupti.\nLaboriosam dolore eius impedit soluta facilis repellat ad quas quia ducimus eum. Suscipit id veritatis veniam, a aliquam possimus laborum atque nemo culpa maiores recusandae assumenda exercitationem quidem consectetur nostrum.",
        ),
    ),
    (
        "jsx",
        1,
        "lorem5*3",
        Some(
            "Lorem ipsum dolor sit amet.\nEum sint id dolores laborum!\nRecusandae sequi porro aperiam rem.",
        ),
    ),
    (
        "jsx",
        1,
        "lorem5*3|bem|c",
        Some(
            "Lorem ipsum dolor sit amet.\nEum sint id dolores laborum!\nRecusandae sequi porro aperiam rem.",
        ),
    ),
    (
        "jsx",
        1,
        "p>lorem5*3",
        Some(
            "<p><span>Lorem ipsum dolor sit amet.</span><span>Eum sint id dolores laborum!</span><span>Recusandae sequi porro aperiam rem.</span></p>",
        ),
    ),
    (
        "jsx",
        1,
        "p>lorem5*3|bem|c",
        Some(
            "<p><span>Lorem ipsum dolor sit amet.</span><span>Eum sint id dolores laborum!</span><span>Recusandae sequi porro aperiam rem.</span></p>",
        ),
    ),
    (
        "jsx",
        1,
        "p*3>lorem12",
        Some(
            "<p>Lorem ipsum dolor sit amet consectetur adipisicing elit. Accusamus eum sint id?</p>\n<p>Amet molestias recusandae sequi porro aperiam rem consequuntur totam? Praesentium, tempore accusamus.</p>\n<p>Necessitatibus porro sapiente quis reiciendis maxime. Deleniti nihil sint consequuntur expedita minima.</p>",
        ),
    ),
    (
        "jsx",
        1,
        "p*3>lorem12|bem|c",
        Some(
            "<p>Lorem ipsum dolor sit amet consectetur adipisicing elit. Accusamus eum sint id?</p>\n<p>Amet molestias recusandae sequi porro aperiam rem consequuntur totam? Praesentium, tempore accusamus.</p>\n<p>Necessitatibus porro sapiente quis reiciendis maxime. Deleniti nihil sint consequuntur expedita minima.</p>",
        ),
    ),
    (
        "jsx",
        1,
        "ul>lorem8*2",
        Some(
            "<ul>\n\t<li>Lorem ipsum dolor sit amet consectetur adipisicing elit.</li>\n\t<li>Eum sint id dolores laborum quam, ex amet?</li>\n</ul>",
        ),
    ),
    (
        "jsx",
        1,
        "ul>lorem8*2|bem|c",
        Some(
            "<ul>\n\t<li>Lorem ipsum dolor sit amet consectetur adipisicing elit.</li>\n\t<li>Eum sint id dolores laborum quam, ex amet?</li>\n</ul>",
        ),
    ),
    (
        "jsx",
        1,
        "ul*2>li>lorem10",
        Some(
            "<ul>\n\t<li>Lorem ipsum dolor sit amet consectetur adipisicing elit. Accusamus, eum?</li>\n</ul>\n<ul>\n\t<li>Amet molestias recusandae sequi porro aperiam rem consequuntur totam? Praesentium?</li>\n</ul>",
        ),
    ),
    (
        "jsx",
        1,
        "ul*2>li>lorem10|bem|c",
        Some(
            "<ul>\n\t<li>Lorem ipsum dolor sit amet consectetur adipisicing elit. Accusamus, eum?</li>\n</ul>\n<ul>\n\t<li>Amet molestias recusandae sequi porro aperiam rem consequuntur totam? Praesentium?</li>\n</ul>",
        ),
    ),
    (
        "jsx",
        1,
        "(h2{title}+p>lorem12)*2",
        Some(
            "<h2>title</h2>\n<p>Lorem ipsum dolor sit amet consectetur adipisicing elit. Accusamus eum sint id?</p>\n<h2>title</h2>\n<p>Amet molestias recusandae sequi porro aperiam rem consequuntur totam? Praesentium, tempore accusamus.</p>",
        ),
    ),
    (
        "jsx",
        1,
        "(h2{title}+p>lorem12)*2|bem|c",
        Some(
            "<h2>title</h2>\n<p>Lorem ipsum dolor sit amet consectetur adipisicing elit. Accusamus eum sint id?</p>\n<h2>title</h2>\n<p>Amet molestias recusandae sequi porro aperiam rem consequuntur totam? Praesentium, tempore accusamus.</p>",
        ),
    ),
    (
        "jsx",
        1,
        "div>lorem5>span{end}",
        Some("<div>Lorem ipsum dolor sit amet.<span>end</span></div>"),
    ),
    (
        "jsx",
        1,
        "div>lorem5>span{end}|bem|c",
        Some("<div>Lorem ipsum dolor sit amet.<span>end</span></div>"),
    ),
    (
        "jsx",
        1,
        "p>lorem5.class#id[title=ignored]",
        Some("<p>Lorem ipsum dolor sit amet.</p>"),
    ),
    (
        "jsx",
        1,
        "p>lorem5.class#id[title=ignored]|bem|c",
        Some("<p>Lorem ipsum dolor sit amet.</p>"),
    ),
    (
        "jsx",
        1,
        "div.block>p.-element>lorem12",
        Some(
            "<div className=\"block\">\n\t<p className=\"-element\">Lorem ipsum dolor sit amet consectetur adipisicing elit. Accusamus eum sint id?</p>\n</div>",
        ),
    ),
    (
        "jsx",
        1,
        "div.block>p.-element>lorem12|bem|c",
        Some(
            "<div className=\"block\">\n\t<p className=\"block__element\">Lorem ipsum dolor sit amet consectetur adipisicing elit. Accusamus eum sint id?</p>\n\t<!-- /.block__element -->\n</div>\n<!-- /.block -->",
        ),
    ),
    (
        "jsx",
        1,
        "lorem5{ignored}",
        Some("Lorem ipsum dolor sit amet."),
    ),
    (
        "jsx",
        1,
        "lorem5{ignored}|bem|c",
        Some("Lorem ipsum dolor sit amet."),
    ),
    (
        "jsx",
        1,
        "loremx5+span",
        Some("Lorem ipsum dolor sit amet.<span>${0}</span>"),
    ),
    (
        "jsx",
        1,
        "loremx5+span|bem|c",
        Some("Lorem ipsum dolor sit amet.<span>${0}</span>"),
    ),
    ("jsx", 1, "lorem5x", Some("<lorem5x>${0}</lorem5x>")),
    ("jsx", 1, "lorem5x|bem|c", Some("<lorem5x>${0}</lorem5x>")),
    ("jsx", 1, "lorem1.5", Some("Lorem.")),
    ("jsx", 1, "lorem1.5|bem|c", Some("Lorem.")),
    (
        "jsx",
        1,
        "lorem2-4-6",
        Some("<lorem2-4-6>${0}</lorem2-4-6>"),
    ),
    (
        "jsx",
        1,
        "lorem2-4-6|bem|c",
        Some("<lorem2-4-6>${0}</lorem2-4-6>"),
    ),
    (
        "jsx",
        1,
        "p>loremru12*2",
        Some(
            "<p><span>Далеко-далеко за словесными горами в стране гласных и согласных живут рыбные тексты. Предупредила, языкового?</span><span>Сбить заголовок путь безорфографичный там коварный грамматики встретил гор? Мир, страна моей.</span></p>",
        ),
    ),
    (
        "jsx",
        1,
        "p>loremru12*2|bem|c",
        Some(
            "<p><span>Далеко-далеко за словесными горами в стране гласных и согласных живут рыбные тексты. Предупредила, языкового?</span><span>Сбить заголовок путь безорфографичный там коварный грамматики встретил гор? Мир, страна моей.</span></p>",
        ),
    ),
    (
        "jsx",
        1,
        "table>loremsp9*2",
        Some(
            "<table>\n\t<tr>Mujer uno dolor más de poder mismo si. Cosas?</tr>\n\t<tr>Dolores laboral manantial, ex entero molestias recusado séquito ventana.</tr>\n</table>",
        ),
    ),
    (
        "jsx",
        1,
        "table>loremsp9*2|bem|c",
        Some(
            "<table>\n\t<tr>Mujer uno dolor más de poder mismo si. Cosas?</tr>\n\t<tr>Dolores laboral manantial, ex entero molestias recusado séquito ventana.</tr>\n</table>",
        ),
    ),
    (
        "jsx",
        42,
        "lorem",
        Some(
            "Lorem ipsum dolor sit amet consectetur adipisicing elit. Iusto accusantium eveniet libero non commodi corrupti doloremque maiores, quam aperiam magni aut dignissimos similique esse dolor voluptates! Beatae nihil repellendus reiciendis?",
        ),
    ),
    (
        "jsx",
        42,
        "lorem|bem|c",
        Some(
            "Lorem ipsum dolor sit amet consectetur adipisicing elit. Iusto accusantium eveniet libero non commodi corrupti doloremque maiores, quam aperiam magni aut dignissimos similique esse dolor voluptates! Beatae nihil repellendus reiciendis?",
        ),
    ),
    ("jsx", 42, "lorem0", Some("Lorem.")),
    ("jsx", 42, "lorem0|bem|c", Some("Lorem.")),
    ("jsx", 42, "lorem1", Some("Lorem.")),
    ("jsx", 42, "lorem1|bem|c", Some("Lorem.")),
    ("jsx", 42, "lorem2", Some("Lorem, ipsum.")),
    ("jsx", 42, "lorem2|bem|c", Some("Lorem, ipsum.")),
    (
        "jsx",
        42,
        "lorem8",
        Some("Lorem ipsum dolor sit amet consectetur adipisicing elit."),
    ),
    (
        "jsx",
        42,
        "lorem8|bem|c",
        Some("Lorem ipsum dolor sit amet consectetur adipisicing elit."),
    ),
    (
        "jsx",
        42,
        "lorem9",
        Some("Lorem ipsum dolor sit amet consectetur adipisicing elit. Iusto!"),
    ),
    (
        "jsx",
        42,
        "lorem9|bem|c",
        Some("Lorem ipsum dolor sit amet consectetur adipisicing elit. Iusto!"),
    ),
    (
        "jsx",
        42,
        "lorem30",
        Some(
            "Lorem ipsum dolor sit amet consectetur adipisicing elit. Iusto accusantium eveniet libero non commodi corrupti doloremque maiores, quam aperiam magni aut dignissimos similique esse dolor voluptates! Beatae nihil repellendus reiciendis?",
        ),
    ),
    (
        "jsx",
        42,
        "lorem30|bem|c",
        Some(
            "Lorem ipsum dolor sit amet consectetur adipisicing elit. Iusto accusantium eveniet libero non commodi corrupti doloremque maiores, quam aperiam magni aut dignissimos similique esse dolor voluptates! Beatae nihil repellendus reiciendis?",
        ),
    ),
    (
        "jsx",
        42,
        "lorem100",
        Some(
            "Lorem ipsum dolor sit amet consectetur adipisicing elit. Iusto accusantium eveniet libero non commodi corrupti doloremque maiores, quam aperiam magni aut dignissimos similique esse dolor voluptates! Beatae nihil repellendus reiciendis officiis eveniet delectus magnam quod earum unde voluptatem cupiditate ut rerum maxime libero quisquam animi tempore at magni fugiat, sapiente natus, veritatis laborum ratione? Quia suscipit provident tempore porro vel est fugiat aliquam ut aliquid non ipsam sed distinctio quibusdam, minima modi eaque cupiditate optio inventore recusandae aspernatur, neque odit! Aliquam nobis consectetur repellendus porro, iste vitae sapiente assumenda minima quod, architecto cumque provident totam tempora aspernatur perspiciatis corporis debitis.",
        ),
    ),
    (
        "jsx",
        42,
        "lorem100|bem|c",
        Some(
            "Lorem ipsum dolor sit amet consectetur adipisicing elit. Iusto accusantium eveniet libero non commodi corrupti doloremque maiores, quam aperiam magni aut dignissimos similique esse dolor voluptates! Beatae nihil repellendus reiciendis officiis eveniet delectus magnam quod earum unde voluptatem cupiditate ut rerum maxime libero quisquam animi tempore at magni fugiat, sapiente natus, veritatis laborum ratione? Quia suscipit provident tempore porro vel est fugiat aliquam ut aliquid non ipsam sed distinctio quibusdam, minima modi eaque cupiditate optio inventore recusandae aspernatur, neque odit! Aliquam nobis consectetur repellendus porro, iste vitae sapiente assumenda minima quod, architecto cumque provident totam tempora aspernatur perspiciatis corporis debitis.",
        ),
    ),
    (
        "jsx",
        42,
        "lorem5-20",
        Some("Lorem ipsum dolor sit amet consectetur adipisicing elit."),
    ),
    (
        "jsx",
        42,
        "lorem5-20|bem|c",
        Some("Lorem ipsum dolor sit amet consectetur adipisicing elit."),
    ),
    (
        "jsx",
        42,
        "lorem20-5",
        Some(
            "Lorem ipsum dolor sit amet consectetur adipisicing elit. Iusto accusantium eveniet libero non commodi corrupti doloremque maiores quam aperiam magni?",
        ),
    ),
    (
        "jsx",
        42,
        "lorem20-5|bem|c",
        Some(
            "Lorem ipsum dolor sit amet consectetur adipisicing elit. Iusto accusantium eveniet libero non commodi corrupti doloremque maiores quam aperiam magni?",
        ),
    ),
    ("jsx", 42, "lorem3-", Some("Lorem, ipsum dolor.")),
    ("jsx", 42, "lorem3-|bem|c", Some("Lorem, ipsum dolor.")),
    (
        "jsx",
        42,
        "loremru12",
        Some(
            "Далеко-далеко за словесными горами в стране гласных и согласных живут рыбные тексты. Текстами, знаках?",
        ),
    ),
    (
        "jsx",
        42,
        "loremru12|bem|c",
        Some(
            "Далеко-далеко за словесными горами в стране гласных и согласных живут рыбные тексты. Текстами, знаках?",
        ),
    ),
    (
        "jsx",
        42,
        "loremsp40",
        Some(
            "Mujer uno dolor más de poder mismo si. Justo accesorio paño libertario no comodidad corrupto paloma mayores, manantial síndrome magnífico auto dignísimo propósito yugo vietnamita fotografía! Corchea nihilista, repelente reincidente oficinista paño, deletéreo magma patrón angustia falda voluntad, encefalograma o?",
        ),
    ),
    (
        "jsx",
        42,
        "loremsp40|bem|c",
        Some(
            "Mujer uno dolor más de poder mismo si. Justo accesorio paño libertario no comodidad corrupto paloma mayores, manantial síndrome magnífico auto dignísimo propósito yugo vietnamita fotografía! Corchea nihilista, repelente reincidente oficinista paño, deletéreo magma patrón angustia falda voluntad, encefalograma o?",
        ),
    ),
    (
        "jsx",
        42,
        "loremfr12",
        Some(
            "Lorem ipsum dolor sit amet consectetur adipisicing elit. Iusto accusantium eveniet libero.",
        ),
    ),
    (
        "jsx",
        42,
        "loremfr12|bem|c",
        Some(
            "Lorem ipsum dolor sit amet consectetur adipisicing elit. Iusto accusantium eveniet libero.",
        ),
    ),
    (
        "jsx",
        42,
        "LOREMRU12",
        Some(
            "Lorem ipsum dolor sit amet consectetur adipisicing elit. Iusto accusantium eveniet libero.",
        ),
    ),
    (
        "jsx",
        42,
        "LOREMRU12|bem|c",
        Some(
            "Lorem ipsum dolor sit amet consectetur adipisicing elit. Iusto accusantium eveniet libero.",
        ),
    ),
    ("jsx", 42, "loremru1", Some("Далеко-далеко.")),
    ("jsx", 42, "loremru1|bem|c", Some("Далеко-далеко.")),
    ("jsx", 42, "loremsp1", Some("Mujer.")),
    ("jsx", 42, "loremsp1|bem|c", Some("Mujer.")),
    (
        "jsx",
        42,
        "lorem0008",
        Some("Lorem ipsum dolor sit amet consectetur adipisicing elit."),
    ),
    (
        "jsx",
        42,
        "lorem0008|bem|c",
        Some("Lorem ipsum dolor sit amet consectetur adipisicing elit."),
    ),
    (
        "jsx",
        42,
        "lorem-8",
        Some(
            "Lorem ipsum dolor sit amet consectetur adipisicing elit. Iusto accusantium eveniet libero non commodi corrupti doloremque maiores, quam aperiam magni aut dignissimos similique esse dolor voluptates! Beatae nihil repellendus reiciendis?",
        ),
    ),
    (
        "jsx",
        42,
        "lorem-8|bem|c",
        Some(
            "Lorem ipsum dolor sit amet consectetur adipisicing elit. Iusto accusantium eveniet libero non commodi corrupti doloremque maiores, quam aperiam magni aut dignissimos similique esse dolor voluptates! Beatae nihil repellendus reiciendis?",
        ),
    ),
    (
        "jsx",
        42,
        "lorem-0",
        Some(
            "Lorem ipsum dolor sit amet consectetur adipisicing elit. Iusto accusantium eveniet libero non commodi corrupti doloremque maiores, quam aperiam magni aut dignissimos similique esse dolor voluptates! Beatae nihil repellendus reiciendis?",
        ),
    ),
    (
        "jsx",
        42,
        "lorem-0|bem|c",
        Some(
            "Lorem ipsum dolor sit amet consectetur adipisicing elit. Iusto accusantium eveniet libero non commodi corrupti doloremque maiores, quam aperiam magni aut dignissimos similique esse dolor voluptates! Beatae nihil repellendus reiciendis?",
        ),
    ),
    (
        "jsx",
        42,
        "lorem*3",
        Some(
            "Lorem ipsum dolor sit amet consectetur adipisicing elit. Iusto accusantium eveniet libero non commodi corrupti doloremque maiores, quam aperiam magni aut dignissimos similique esse dolor voluptates! Beatae nihil repellendus reiciendis?\nQuod earum unde voluptatem cupiditate ut rerum maxime, libero quisquam animi tempore at magni fugiat sapiente natus veritatis laborum ratione necessitatibus optio eos cumque asperiores? Vel est fugiat aliquam ut?\nSed distinctio quibusdam minima modi. Inventore cupiditate recusandae aspernatur neque odit ducimus, quae facere quo, ad aliquam nobis consectetur repellendus porro iste vitae sapiente assumenda minima quod architecto cumque provident?",
        ),
    ),
    (
        "jsx",
        42,
        "lorem*3|bem|c",
        Some(
            "Lorem ipsum dolor sit amet consectetur adipisicing elit. Iusto accusantium eveniet libero non commodi corrupti doloremque maiores, quam aperiam magni aut dignissimos similique esse dolor voluptates! Beatae nihil repellendus reiciendis?\nQuod earum unde voluptatem cupiditate ut rerum maxime, libero quisquam animi tempore at magni fugiat sapiente natus veritatis laborum ratione necessitatibus optio eos cumque asperiores? Vel est fugiat aliquam ut?\nSed distinctio quibusdam minima modi. Inventore cupiditate recusandae aspernatur neque odit ducimus, quae facere quo, ad aliquam nobis consectetur repellendus porro iste vitae sapiente assumenda minima quod architecto cumque provident?",
        ),
    ),
    (
        "jsx",
        42,
        "lorem5*3",
        Some(
            "Lorem ipsum dolor sit amet.\nAccusantium eveniet libero non commodi.\nAperiam magni aut dignissimos similique.",
        ),
    ),
    (
        "jsx",
        42,
        "lorem5*3|bem|c",
        Some(
            "Lorem ipsum dolor sit amet.\nAccusantium eveniet libero non commodi.\nAperiam magni aut dignissimos similique.",
        ),
    ),
    (
        "jsx",
        42,
        "p>lorem5*3",
        Some(
            "<p><span>Lorem ipsum dolor sit amet.</span><span>Accusantium eveniet libero non commodi.</span><span>Aperiam magni aut dignissimos similique.</span></p>",
        ),
    ),
    (
        "jsx",
        42,
        "p>lorem5*3|bem|c",
        Some(
            "<p><span>Lorem ipsum dolor sit amet.</span><span>Accusantium eveniet libero non commodi.</span><span>Aperiam magni aut dignissimos similique.</span></p>",
        ),
    ),
    (
        "jsx",
        42,
        "p*3>lorem12",
        Some(
            "<p>Lorem ipsum dolor sit amet consectetur adipisicing elit. Iusto accusantium eveniet libero.</p>\n<p>Maiores quam aperiam magni aut dignissimos similique esse dolor, voluptates ipsum voluptatibus?</p>\n<p>Officiis eveniet delectus magnam quod earum unde voluptatem cupiditate ut rerum maxime?</p>",
        ),
    ),
    (
        "jsx",
        42,
        "p*3>lorem12|bem|c",
        Some(
            "<p>Lorem ipsum dolor sit amet consectetur adipisicing elit. Iusto accusantium eveniet libero.</p>\n<p>Maiores quam aperiam magni aut dignissimos similique esse dolor, voluptates ipsum voluptatibus?</p>\n<p>Officiis eveniet delectus magnam quod earum unde voluptatem cupiditate ut rerum maxime?</p>",
        ),
    ),
    (
        "jsx",
        42,
        "ul>lorem8*2",
        Some(
            "<ul>\n\t<li>Lorem ipsum dolor sit amet consectetur adipisicing elit.</li>\n\t<li>Accusantium eveniet libero non commodi corrupti, doloremque maiores.</li>\n</ul>",
        ),
    ),
    (
        "jsx",
        42,
        "ul>lorem8*2|bem|c",
        Some(
            "<ul>\n\t<li>Lorem ipsum dolor sit amet consectetur adipisicing elit.</li>\n\t<li>Accusantium eveniet libero non commodi corrupti, doloremque maiores.</li>\n</ul>",
        ),
    ),
    (
        "jsx",
        42,
        "ul*2>li>lorem10",
        Some(
            "<ul>\n\t<li>Lorem ipsum dolor sit amet consectetur adipisicing elit. Iusto, accusantium?</li>\n</ul>\n<ul>\n\t<li>Doloremque maiores, quam aperiam magni aut dignissimos similique esse dolor.</li>\n</ul>",
        ),
    ),
    (
        "jsx",
        42,
        "ul*2>li>lorem10|bem|c",
        Some(
            "<ul>\n\t<li>Lorem ipsum dolor sit amet consectetur adipisicing elit. Iusto, accusantium?</li>\n</ul>\n<ul>\n\t<li>Doloremque maiores, quam aperiam magni aut dignissimos similique esse dolor.</li>\n</ul>",
        ),
    ),
    (
        "jsx",
        42,
        "(h2{title}+p>lorem12)*2",
        Some(
            "<h2>title</h2>\n<p>Lorem ipsum dolor sit amet consectetur adipisicing elit. Iusto accusantium eveniet libero.</p>\n<h2>title</h2>\n<p>Maiores quam aperiam magni aut dignissimos similique esse dolor, voluptates ipsum voluptatibus?</p>",
        ),
    ),
    (
        "jsx",
        42,
        "(h2{title}+p>lorem12)*2|bem|c",
        Some(
            "<h2>title</h2>\n<p>Lorem ipsum dolor sit amet consectetur adipisicing elit. Iusto accusantium eveniet libero.</p>\n<h2>title</h2>\n<p>Maiores quam aperiam magni aut dignissimos similique esse dolor, voluptates ipsum voluptatibus?</p>",
        ),
    ),
    (
        "jsx",
        42,
        "div>lorem5>span{end}",
        Some("<div>Lorem ipsum dolor sit amet.<span>end</span></div>"),
    ),
    (
        "jsx",
        42,
        "div>lorem5>span{end}|bem|c",
        Some("<div>Lorem ipsum dolor sit amet.<span>end</span></div>"),
    ),
    (
        "jsx",
        42,
        "p>lorem5.class#id[title=ignored]",
        Some("<p>Lorem ipsum dolor sit amet.</p>"),
    ),
    (
        "jsx",
        42,
        "p>lorem5.class#id[title=ignored]|bem|c",
        Some("<p>Lorem ipsum dolor sit amet.</p>"),
    ),
    (
        "jsx",
        42,
        "div.block>p.-element>lorem12",
        Some(
            "<div className=\"block\">\n\t<p className=\"-element\">Lorem ipsum dolor sit amet consectetur adipisicing elit. Iusto accusantium eveniet libero.</p>\n</div>",
        ),
    ),
    (
        "jsx",
        42,
        "div.block>p.-element>lorem12|bem|c",
        Some(
            "<div className=\"block\">\n\t<p className=\"block__element\">Lorem ipsum dolor sit amet consectetur adipisicing elit. Iusto accusantium eveniet libero.</p>\n\t<!-- /.block__element -->\n</div>\n<!-- /.block -->",
        ),
    ),
    (
        "jsx",
        42,
        "lorem5{ignored}",
        Some("Lorem ipsum dolor sit amet."),
    ),
    (
        "jsx",
        42,
        "lorem5{ignored}|bem|c",
        Some("Lorem ipsum dolor sit amet."),
    ),
    (
        "jsx",
        42,
        "loremx5+span",
        Some("Lorem ipsum dolor sit amet.<span>${0}</span>"),
    ),
    (
        "jsx",
        42,
        "loremx5+span|bem|c",
        Some("Lorem ipsum dolor sit amet.<span>${0}</span>"),
    ),
    ("jsx", 42, "lorem5x", Some("<lorem5x>${0}</lorem5x>")),
    ("jsx", 42, "lorem5x|bem|c", Some("<lorem5x>${0}</lorem5x>")),
    ("jsx", 42, "lorem1.5", Some("Lorem.")),
    ("jsx", 42, "lorem1.5|bem|c", Some("Lorem.")),
    (
        "jsx",
        42,
        "lorem2-4-6",
        Some("<lorem2-4-6>${0}</lorem2-4-6>"),
    ),
    (
        "jsx",
        42,
        "lorem2-4-6|bem|c",
        Some("<lorem2-4-6>${0}</lorem2-4-6>"),
    ),
    (
        "jsx",
        42,
        "p>loremru12*2",
        Some(
            "<p><span>Далеко-далеко за словесными горами в стране гласных и согласных живут рыбные тексты. Текстами, знаках?</span><span>Повстречался пустился скатился, коварный деревни обеспечивает которой проектах использовало языком взгляд всемогущая.</span></p>",
        ),
    ),
    (
        "jsx",
        42,
        "p>loremru12*2|bem|c",
        Some(
            "<p><span>Далеко-далеко за словесными горами в стране гласных и согласных живут рыбные тексты. Текстами, знаках?</span><span>Повстречался пустился скатился, коварный деревни обеспечивает которой проектах использовало языком взгляд всемогущая.</span></p>",
        ),
    ),
    (
        "jsx",
        42,
        "table>loremsp9*2",
        Some(
            "<table>\n\t<tr>Mujer uno dolor más de poder mismo si. Justo!</tr>\n\t<tr>No comodidad corrupto paloma mayores manantial síndrome magnífico auto.</tr>\n</table>",
        ),
    ),
    (
        "jsx",
        42,
        "table>loremsp9*2|bem|c",
        Some(
            "<table>\n\t<tr>Mujer uno dolor más de poder mismo si. Justo!</tr>\n\t<tr>No comodidad corrupto paloma mayores manantial síndrome magnífico auto.</tr>\n</table>",
        ),
    ),
    (
        "jsx",
        987654321,
        "lorem",
        Some(
            "Lorem ipsum dolor sit, amet consectetur adipisicing elit. Deserunt voluptas quos nam, nisi nesciunt mollitia veritatis odit hic dolores, nemo ex. Quod molestias ex rerum, natus amet voluptatum fugiat mollitia.",
        ),
    ),
    (
        "jsx",
        987654321,
        "lorem|bem|c",
        Some(
            "Lorem ipsum dolor sit, amet consectetur adipisicing elit. Deserunt voluptas quos nam, nisi nesciunt mollitia veritatis odit hic dolores, nemo ex. Quod molestias ex rerum, natus amet voluptatum fugiat mollitia.",
        ),
    ),
    ("jsx", 987654321, "lorem0", Some("Lorem.")),
    ("jsx", 987654321, "lorem0|bem|c", Some("Lorem.")),
    ("jsx", 987654321, "lorem1", Some("Lorem.")),
    ("jsx", 987654321, "lorem1|bem|c", Some("Lorem.")),
    ("jsx", 987654321, "lorem2", Some("Lorem, ipsum.")),
    ("jsx", 987654321, "lorem2|bem|c", Some("Lorem, ipsum.")),
    (
        "jsx",
        987654321,
        "lorem8",
        Some("Lorem ipsum dolor sit, amet consectetur adipisicing elit."),
    ),
    (
        "jsx",
        987654321,
        "lorem8|bem|c",
        Some("Lorem ipsum dolor sit, amet consectetur adipisicing elit."),
    ),
    (
        "jsx",
        987654321,
        "lorem9",
        Some("Lorem ipsum dolor sit, amet consectetur adipisicing elit. Deserunt!"),
    ),
    (
        "jsx",
        987654321,
        "lorem9|bem|c",
        Some("Lorem ipsum dolor sit, amet consectetur adipisicing elit. Deserunt!"),
    ),
    (
        "jsx",
        987654321,
        "lorem30",
        Some(
            "Lorem ipsum dolor sit, amet consectetur adipisicing elit. Deserunt voluptas quos nam, nisi nesciunt mollitia veritatis odit hic dolores, nemo ex. Quod molestias ex rerum, natus amet voluptatum fugiat mollitia.",
        ),
    ),
    (
        "jsx",
        987654321,
        "lorem30|bem|c",
        Some(
            "Lorem ipsum dolor sit, amet consectetur adipisicing elit. Deserunt voluptas quos nam, nisi nesciunt mollitia veritatis odit hic dolores, nemo ex. Quod molestias ex rerum, natus amet voluptatum fugiat mollitia.",
        ),
    ),
    (
        "jsx",
        987654321,
        "lorem100",
        Some(
            "Lorem ipsum dolor sit, amet consectetur adipisicing elit. Deserunt voluptas quos nam, nisi nesciunt mollitia veritatis odit hic dolores, nemo ex. Quod molestias ex rerum natus amet voluptatum fugiat mollitia nemo delectus illum harum aperiam maiores praesentium quasi, vero hic temporibus doloremque porro beatae? Obcaecati aperiam, a eum sapiente fuga, maiores inventore dolor accusamus quis reiciendis impedit deleniti, doloribus voluptatibus soluta id vero velit dolorem ipsa incidunt. Sit beatae facilis, totam sint quod quaerat sunt molestiae culpa optio quam aperiam libero delectus voluptates incidunt ea! Doloribus assumenda unde tempore tenetur animi? Aut quod molestiae suscipit esse adipisci. Perferendis, incidunt odit.",
        ),
    ),
    (
        "jsx",
        987654321,
        "lorem100|bem|c",
        Some(
            "Lorem ipsum dolor sit, amet consectetur adipisicing elit. Deserunt voluptas quos nam, nisi nesciunt mollitia veritatis odit hic dolores, nemo ex. Quod molestias ex rerum natus amet voluptatum fugiat mollitia nemo delectus illum harum aperiam maiores praesentium quasi, vero hic temporibus doloremque porro beatae? Obcaecati aperiam, a eum sapiente fuga, maiores inventore dolor accusamus quis reiciendis impedit deleniti, doloribus voluptatibus soluta id vero velit dolorem ipsa incidunt. Sit beatae facilis, totam sint quod quaerat sunt molestiae culpa optio quam aperiam libero delectus voluptates incidunt ea! Doloribus assumenda unde tempore tenetur animi? Aut quod molestiae suscipit esse adipisci. Perferendis, incidunt odit.",
        ),
    ),
    (
        "jsx",
        987654321,
        "lorem5-20",
        Some("Lorem ipsum dolor sit amet."),
    ),
    (
        "jsx",
        987654321,
        "lorem5-20|bem|c",
        Some("Lorem ipsum dolor sit amet."),
    ),
    (
        "jsx",
        987654321,
        "lorem20-5",
        Some(
            "Lorem ipsum dolor sit, amet consectetur adipisicing elit. Deserunt voluptas quos nam nisi nesciunt mollitia veritatis odit hic dolores nemo.",
        ),
    ),
    (
        "jsx",
        987654321,
        "lorem20-5|bem|c",
        Some(
            "Lorem ipsum dolor sit, amet consectetur adipisicing elit. Deserunt voluptas quos nam nisi nesciunt mollitia veritatis odit hic dolores nemo.",
        ),
    ),
    ("jsx", 987654321, "lorem3-", Some("Lorem, ipsum dolor.")),
    (
        "jsx",
        987654321,
        "lorem3-|bem|c",
        Some("Lorem, ipsum dolor."),
    ),
    (
        "jsx",
        987654321,
        "loremru12",
        Some(
            "Далеко-далеко за словесными горами в стране, гласных и согласных живут рыбные тексты. Свой, злых.",
        ),
    ),
    (
        "jsx",
        987654321,
        "loremru12|bem|c",
        Some(
            "Далеко-далеко за словесными горами в стране, гласных и согласных живут рыбные тексты. Свой, злых.",
        ),
    ),
    (
        "jsx",
        987654321,
        "loremsp40",
        Some(
            "Mujer uno dolor más, de poder mismo si. Desierto voluble pan namibia, león nascimiento computador verdades odio alma dolores, remo ex. Patrón molestias ex renuncia natividad entero torre refugiado computador remo deletéreo marejada olvido síndrome, mayores, presentación casi, alveolar alma?",
        ),
    ),
    (
        "jsx",
        987654321,
        "loremsp40|bem|c",
        Some(
            "Mujer uno dolor más, de poder mismo si. Desierto voluble pan namibia, león nascimiento computador verdades odio alma dolores, remo ex. Patrón molestias ex renuncia natividad entero torre refugiado computador remo deletéreo marejada olvido síndrome, mayores, presentación casi, alveolar alma?",
        ),
    ),
    (
        "jsx",
        987654321,
        "loremfr12",
        Some(
            "Lorem ipsum dolor sit, amet consectetur adipisicing elit. Deserunt voluptas quos nam!",
        ),
    ),
    (
        "jsx",
        987654321,
        "loremfr12|bem|c",
        Some(
            "Lorem ipsum dolor sit, amet consectetur adipisicing elit. Deserunt voluptas quos nam!",
        ),
    ),
    (
        "jsx",
        987654321,
        "LOREMRU12",
        Some(
            "Lorem ipsum dolor sit, amet consectetur adipisicing elit. Deserunt voluptas quos nam!",
        ),
    ),
    (
        "jsx",
        987654321,
        "LOREMRU12|bem|c",
        Some(
            "Lorem ipsum dolor sit, amet consectetur adipisicing elit. Deserunt voluptas quos nam!",
        ),
    ),
    ("jsx", 987654321, "loremru1", Some("Далеко-далеко.")),
    ("jsx", 987654321, "loremru1|bem|c", Some("Далеко-далеко.")),
    ("jsx", 987654321, "loremsp1", Some("Mujer.")),
    ("jsx", 987654321, "loremsp1|bem|c", Some("Mujer.")),
    (
        "jsx",
        987654321,
        "lorem0008",
        Some("Lorem ipsum dolor sit, amet consectetur adipisicing elit."),
    ),
    (
        "jsx",
        987654321,
        "lorem0008|bem|c",
        Some("Lorem ipsum dolor sit, amet consectetur adipisicing elit."),
    ),
    (
        "jsx",
        987654321,
        "lorem-8",
        Some(
            "Lorem ipsum dolor sit, amet consectetur adipisicing elit. Deserunt voluptas quos nam, nisi nesciunt mollitia veritatis odit hic dolores, nemo ex. Quod molestias ex rerum, natus amet voluptatum fugiat mollitia.",
        ),
    ),
    (
        "jsx",
        987654321,
        "lorem-8|bem|c",
        Some(
            "Lorem ipsum dolor sit, amet consectetur adipisicing elit. Deserunt voluptas quos nam, nisi nesciunt mollitia veritatis odit hic dolores, nemo ex. Quod molestias ex rerum, natus amet voluptatum fugiat mollitia.",
        ),
    ),
    (
        "jsx",
        987654321,
        "lorem-0",
        Some(
            "Lorem ipsum dolor sit, amet consectetur adipisicing elit. Deserunt voluptas quos nam, nisi nesciunt mollitia veritatis odit hic dolores, nemo ex. Quod molestias ex rerum, natus amet voluptatum fugiat mollitia.",
        ),
    ),
    (
        "jsx",
        987654321,
        "lorem-0|bem|c",
        Some(
            "Lorem ipsum dolor sit, amet consectetur adipisicing elit. Deserunt voluptas quos nam, nisi nesciunt mollitia veritatis odit hic dolores, nemo ex. Quod molestias ex rerum, natus amet voluptatum fugiat mollitia.",
        ),
    ),
    (
        "jsx",
        987654321,
        "lorem*3",
        Some(
            "Lorem ipsum dolor sit, amet consectetur adipisicing elit. Deserunt voluptas quos nam, nisi nesciunt mollitia veritatis odit hic dolores, nemo ex. Quod molestias ex rerum, natus amet voluptatum fugiat mollitia.\nMaiores praesentium quasi vero hic temporibus doloremque, porro, beatae velit at iusto nemo obcaecati aperiam a eum sapiente fuga inventore dolor accusamus quis reiciendis impedit deleniti! Vero velit dolorem ipsa.\nSed distinctio omnis sit beatae facilis totam sint quod quaerat sunt, molestiae culpa optio quam aperiam libero delectus voluptates! Autem facilis doloribus assumenda unde. Porro quisquam facilis aut quod molestiae.",
        ),
    ),
    (
        "jsx",
        987654321,
        "lorem*3|bem|c",
        Some(
            "Lorem ipsum dolor sit, amet consectetur adipisicing elit. Deserunt voluptas quos nam, nisi nesciunt mollitia veritatis odit hic dolores, nemo ex. Quod molestias ex rerum, natus amet voluptatum fugiat mollitia.\nMaiores praesentium quasi vero hic temporibus doloremque, porro, beatae velit at iusto nemo obcaecati aperiam a eum sapiente fuga inventore dolor accusamus quis reiciendis impedit deleniti! Vero velit dolorem ipsa.\nSed distinctio omnis sit beatae facilis totam sint quod quaerat sunt, molestiae culpa optio quam aperiam libero delectus voluptates! Autem facilis doloribus assumenda unde. Porro quisquam facilis aut quod molestiae.",
        ),
    ),
    (
        "jsx",
        987654321,
        "lorem5*3",
        Some(
            "Lorem ipsum dolor sit amet.\nDeserunt voluptas quos nam nisi.\nHic dolores nemo ex cupiditate!",
        ),
    ),
    (
        "jsx",
        987654321,
        "lorem5*3|bem|c",
        Some(
            "Lorem ipsum dolor sit amet.\nDeserunt voluptas quos nam nisi.\nHic dolores nemo ex cupiditate!",
        ),
    ),
    (
        "jsx",
        987654321,
        "p>lorem5*3",
        Some(
            "<p><span>Lorem ipsum dolor sit amet.</span><span>Deserunt voluptas quos nam nisi.</span><span>Hic dolores nemo ex cupiditate!</span></p>",
        ),
    ),
    (
        "jsx",
        987654321,
        "p>lorem5*3|bem|c",
        Some(
            "<p><span>Lorem ipsum dolor sit amet.</span><span>Deserunt voluptas quos nam nisi.</span><span>Hic dolores nemo ex cupiditate!</span></p>",
        ),
    ),
    (
        "jsx",
        987654321,
        "p*3>lorem12",
        Some(
            "<p>Lorem ipsum dolor sit, amet consectetur adipisicing elit. Deserunt voluptas quos nam!</p>\n<p>Odit hic dolores nemo ex cupiditate, praesentium esse quia. Rerum, natus amet.</p>\n<p>Harum aperiam maiores, praesentium quasi vero hic temporibus doloremque porro beatae velit.</p>",
        ),
    ),
    (
        "jsx",
        987654321,
        "p*3>lorem12|bem|c",
        Some(
            "<p>Lorem ipsum dolor sit, amet consectetur adipisicing elit. Deserunt voluptas quos nam!</p>\n<p>Odit hic dolores nemo ex cupiditate, praesentium esse quia. Rerum, natus amet.</p>\n<p>Harum aperiam maiores, praesentium quasi vero hic temporibus doloremque porro beatae velit.</p>",
        ),
    ),
    (
        "jsx",
        987654321,
        "ul>lorem8*2",
        Some(
            "<ul>\n\t<li>Lorem ipsum dolor sit, amet consectetur adipisicing elit.</li>\n\t<li>Voluptas quos nam nisi, nesciunt mollitia veritatis odit.</li>\n</ul>",
        ),
    ),
    (
        "jsx",
        987654321,
        "ul>lorem8*2|bem|c",
        Some(
            "<ul>\n\t<li>Lorem ipsum dolor sit, amet consectetur adipisicing elit.</li>\n\t<li>Voluptas quos nam nisi, nesciunt mollitia veritatis odit.</li>\n</ul>",
        ),
    ),
    (
        "jsx",
        987654321,
        "ul*2>li>lorem10",
        Some(
            "<ul>\n\t<li>Lorem ipsum dolor sit, amet consectetur adipisicing elit. Deserunt, voluptas.</li>\n</ul>\n<ul>\n\t<li>Hic dolores nemo ex cupiditate! Quia at quod molestias ex?</li>\n</ul>",
        ),
    ),
    (
        "jsx",
        987654321,
        "ul*2>li>lorem10|bem|c",
        Some(
            "<ul>\n\t<li>Lorem ipsum dolor sit, amet consectetur adipisicing elit. Deserunt, voluptas.</li>\n</ul>\n<ul>\n\t<li>Hic dolores nemo ex cupiditate! Quia at quod molestias ex?</li>\n</ul>",
        ),
    ),
    (
        "jsx",
        987654321,
        "(h2{title}+p>lorem12)*2",
        Some(
            "<h2>title</h2>\n<p>Lorem ipsum dolor sit, amet consectetur adipisicing elit. Deserunt voluptas quos nam!</p>\n<h2>title</h2>\n<p>Odit hic dolores nemo ex cupiditate, praesentium esse quia. Rerum, natus amet.</p>",
        ),
    ),
    (
        "jsx",
        987654321,
        "(h2{title}+p>lorem12)*2|bem|c",
        Some(
            "<h2>title</h2>\n<p>Lorem ipsum dolor sit, amet consectetur adipisicing elit. Deserunt voluptas quos nam!</p>\n<h2>title</h2>\n<p>Odit hic dolores nemo ex cupiditate, praesentium esse quia. Rerum, natus amet.</p>",
        ),
    ),
    (
        "jsx",
        987654321,
        "div>lorem5>span{end}",
        Some("<div>Lorem ipsum dolor sit amet.<span>end</span></div>"),
    ),
    (
        "jsx",
        987654321,
        "div>lorem5>span{end}|bem|c",
        Some("<div>Lorem ipsum dolor sit amet.<span>end</span></div>"),
    ),
    (
        "jsx",
        987654321,
        "p>lorem5.class#id[title=ignored]",
        Some("<p>Lorem ipsum dolor sit amet.</p>"),
    ),
    (
        "jsx",
        987654321,
        "p>lorem5.class#id[title=ignored]|bem|c",
        Some("<p>Lorem ipsum dolor sit amet.</p>"),
    ),
    (
        "jsx",
        987654321,
        "div.block>p.-element>lorem12",
        Some(
            "<div className=\"block\">\n\t<p className=\"-element\">Lorem ipsum dolor sit, amet consectetur adipisicing elit. Deserunt voluptas quos nam!</p>\n</div>",
        ),
    ),
    (
        "jsx",
        987654321,
        "div.block>p.-element>lorem12|bem|c",
        Some(
            "<div className=\"block\">\n\t<p className=\"block__element\">Lorem ipsum dolor sit, amet consectetur adipisicing elit. Deserunt voluptas quos nam!</p>\n\t<!-- /.block__element -->\n</div>\n<!-- /.block -->",
        ),
    ),
    (
        "jsx",
        987654321,
        "lorem5{ignored}",
        Some("Lorem ipsum dolor sit amet."),
    ),
    (
        "jsx",
        987654321,
        "lorem5{ignored}|bem|c",
        Some("Lorem ipsum dolor sit amet."),
    ),
    (
        "jsx",
        987654321,
        "loremx5+span",
        Some("Lorem ipsum dolor sit amet.<span>${0}</span>"),
    ),
    (
        "jsx",
        987654321,
        "loremx5+span|bem|c",
        Some("Lorem ipsum dolor sit amet.<span>${0}</span>"),
    ),
    ("jsx", 987654321, "lorem5x", Some("<lorem5x>${0}</lorem5x>")),
    (
        "jsx",
        987654321,
        "lorem5x|bem|c",
        Some("<lorem5x>${0}</lorem5x>"),
    ),
    ("jsx", 987654321, "lorem1.5", Some("Lorem.")),
    ("jsx", 987654321, "lorem1.5|bem|c", Some("Lorem.")),
    (
        "jsx",
        987654321,
        "lorem2-4-6",
        Some("<lorem2-4-6>${0}</lorem2-4-6>"),
    ),
    (
        "jsx",
        987654321,
        "lorem2-4-6|bem|c",
        Some("<lorem2-4-6>${0}</lorem2-4-6>"),
    ),
    (
        "jsx",
        987654321,
        "p>loremru12*2",
        Some(
            "<p><span>Далеко-далеко за словесными горами в стране, гласных и согласных живут рыбные тексты. Свой, злых.</span><span>Рукопись назад меня однажды она! Города это вскоре заголовок однажды все рыбными.</span></p>",
        ),
    ),
    (
        "jsx",
        987654321,
        "p>loremru12*2|bem|c",
        Some(
            "<p><span>Далеко-далеко за словесными горами в стране, гласных и согласных живут рыбные тексты. Свой, злых.</span><span>Рукопись назад меня однажды она! Города это вскоре заголовок однажды все рыбными.</span></p>",
        ),
    ),
    (
        "jsx",
        987654321,
        "table>loremsp9*2",
        Some(
            "<table>\n\t<tr>Mujer uno dolor más, de poder mismo si. Desierto!</tr>\n\t<tr>León, nascimiento computador. Remo ex encefalograma presentación yugo resiliencia.</tr>\n</table>",
        ),
    ),
    (
        "jsx",
        987654321,
        "table>loremsp9*2|bem|c",
        Some(
            "<table>\n\t<tr>Mujer uno dolor más, de poder mismo si. Desierto!</tr>\n\t<tr>León, nascimiento computador. Remo ex encefalograma presentación yugo resiliencia.</tr>\n</table>",
        ),
    ),
];
