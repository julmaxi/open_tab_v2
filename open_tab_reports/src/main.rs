use allsorts::{tag::LATN, font::MatchingPresentation};
use itertools::Itertools;
use open_tab_reports::layout::{LayoutedDocument, font::FontLoader, LayoutedPage, PageDimensions, Instruction, design::{DocumentLayouter, TextLayouter, QRCodeLayouter, TabularLayouter, CellInfo, RowInfo}};



fn main() {
    let loader = FontLoader::new();
    let menlo = loader.load_from_postscript_name("Menlo".to_string()).unwrap();
    

    let mut doc = LayoutedDocument::new();
    let mut allsorts_font = menlo.as_allsorts();

    let glyphs = allsorts_font.map_glyphs("Shaping in a jiffy.", LATN, MatchingPresentation::NotRequired);
    let glyphs = glyphs.into_iter().map(|g| g.glyph_index).collect::<Vec<_>>();

    drop(allsorts_font);

    let font_ref = doc.add_font(menlo);
    let n_glyphs = glyphs.len();

    let mut page: LayoutedPage = LayoutedPage::new(PageDimensions::a4());
    page.add_element(open_tab_reports::layout::LayoutedElement::Text(open_tab_reports::layout::TextElement {
        font: font_ref,
        font_size: 12.0,
        glyph_ids: glyphs,
        instructions: vec![
            Instruction::MoveTo { x: 0.0, y: 10.0 },
            Instruction::Run { start: 0, stop: n_glyphs },
        ],
    }));
    doc.add_page(page);

    let data = doc.write_as_pdf().unwrap();
    //Write to test.pdf
    std::fs::write("test-new.pdf", data).unwrap();


    let mut content = DocumentLayouter::new();
    let _text = TextLayouter {
        //text: "ABCD EFGH IJKL MNOP QRST UVWX YZ".chars().into_iter().cycle().take(5000).join("").to_string(),
        text: "Zu Dionys dem Tyrannen, schlich 
        Damon den Dolch im Gewande, 
        Ihn schlugen die Häscher in Bande. 
        „Was wolltest du mit dem Dolche, sprich!“
        Entgegnet ihm finster der Wüterich. 
        „Die Stadt vom Tyrannen befreien!“ 
        Das sollst du am Kreuze bereuen.“
        
        „Ich bin“, spricht jener, „zu sterben bereit, 
        Und bitte nicht um mein Leben; 
        Doch willst du Gnade mir geben, 
        Ich flehe dich um drei Tage Zeit, 
        Bis ich die Schwester dem Gatten gefreit:
        Ich lasse den Freund dir als Bürgen –
        Ihn magst du, entrinn ich, erwürgen.“
        
        Da lächelt der König mit arger List 
        Und spricht nach kurzem Bedenken: 
        „Drei Tage will ich dir schenken. 
        Doch wisse: wenn sie verstrichen, die Frist, 
        Eh du zurück mir gegeben bist, 
        So muß er statt deiner erblassen, 
        Doch dir ist die Strafe erlassen.“
        
        Und er kommt zum Freunde: „Der König gebeut, 
        Daß ich am Kreuz mit dem Leben 
        Bezahle das frevelnde Streben 
        Doch will er mir gönnen drei Tage Zeit, 
        Bis ich die Schwester dem Gatten gefreit, 
        So bleib du dem König zum Pfande, 
        Bis ich komme, zu lösen die Bande.“
        
        Und schweigend umarmt ihn der treue Freund, 
        Und liefert sich aus dem Tyrannen, 
        Der andere ziehet von dannen. 
        Und ehe das dritte Morgenrot scheint, 
        Hat er schnell mit dem Gatten die Schwester vereint, 
        Eilt heim mit sorgender Seele, 
        Damit er die Frist nicht verfehle.
        
        Da gießt unendlicher Regen herab, 
        Von den Bergen stürzen die Quellen, 
        Und die Bäche, die Ströme schwellen. 
        Und er kommt ans Ufer mit wanderndem Stab – 
        Da reißet die Brücke der Strudel hinab, 
        Und donnernd sprengen die Wogen 
        Des Gewölbes krachenden Bogen.
        
        Und trostlos irrt er an Ufers Rand, 
        Wie weit er auch spähet und blicket, 
        Und die Stimme, die rufende, schicket – 
        Da stößet kein Nachen vom sichern Strand, 
        Der ihn setze an das gewünschte Land, 
        Kein Schiffer lenket die Fähre, 
        Und der wilde Strom wird zum Meere.
        
        Da sinkt er ans Ufer und weint und fleht, 
        Die Hände zum Zeus erhoben: 
        „O hemme des Stromes Toben! 
        Es eilen die Stunden, im Mittag steht 
        Die Sonne und wenn sie niedergeht, 
        Und ich kann die Stadt nicht erreichen, 
        So muß der Freund mir erbleichen.“
        
        Doch wachsend erneut sich des Stromes Wut, 
        Und Welle auf Welle zerrinnet, 
        Und Stunde an Stunde entrinnet, 
        Da treibt ihn die Angst, da faßt er sich Mut 
        Und wirft sich hinein in die brausende Flut, 
        Und teilt mit gewaltigen Armen 
        Den Strom, und ein Gott hat Erbarmen.
        
        Und gewinnt das Ufer und eilet fort, 
        Und danket dem rettenden Gotte; 
        Da stürzet die raubende Rotte 
        Hervor aus des Waldes nächtlichem Ort, 
        Den Pfad ihm sperrend, und schnaubet Mord 
        Und hemmet des Wanderers Eile 
        Mit drohend geschwungener Keule.
        
        „Was wollt ihr?“ ruft er vor Schrecken bleich 
        „Ich habe nichts als mein Leben, 
        Das muß ich dem Könige geben!“
        Und entreißt die Keule dem nächsten gleich: 
        „Um des Freundes willen erbarmet euch!“
        Und drei, mit gewaltigen Streichen, 
        Erlegt er, die andern entweichen.
        
        Und die Sonne versendet glühenden Brand 
        Und von der unendlichen Mühe 
        Ermattet sinken die Kniee: 
        „O hast du mich gnädig aus Räubershand, 
        Aus dem Strom mich gerettet ans heilige Land, 
        Und soll hier verschmachtend verderben, 
        Und der Freund mir, der liebende, sterben!“
        
        Und horch! da sprudelt es silberhell 
        Ganz nahe, wie rieselndes Rauschen, 
        Und stille hält er, zu lauschen;
        Und sieh, aus dem Felsen, geschwätzig, schnell, 
        Springt murmelnd hervor ein lebendiger Quell, 
        Und freudig bückt er sich nieder, 
        Und erfrischet die brennenden Glieder.
        
        Und die Sonne blickt durch der Zweige Grün 
        Und malt auf den glänzenden Matten 
        Der Bäume gigantische Schatten; 
        Und zwei Wanderer sieht er die Straße ziehn, 
        Will eilenden Laufes vorüber fliehn, 
        Da hört er die Worte sie sagen: 
        „Jetzt wird er ans Kreuz geschlagen.“
        
        Und die Angst beflügelt den eilenden Fuß, 
        Ihn jagen der Sorge Qualen; 
        Da schimmern in Abendrots Strahlen 
        Von ferne die Zinnen von Syrakus, 
        Und entgegen kommt ihm Philostratus, 
        Des Hauses redlicher Hüter, 
        Der erkennet entsetzt den Gebieter:
        
        „Zurück! du rettest den Freund nicht mehr, 
        So rette das eigene Leben! 
        Den Tod erleidet er eben. 
        Von Stunde zu Stunde gewartet' er 
        Mit hoffender Seele der Wiederkehr, 
        Ihm konnte den mutigen Glauben 
        Der Hohn des Tyrannen nicht rauben.“
        
        „Und ist es zu spät, und kann ich ihm nicht
        Ein Retter willkommen erscheinen, 
        So soll mich der Tod ihm vereinen. 
        Des rühme der blutge Tyrann sich nicht, 
        Daß der Freund dem Freunde gebrochen die Pflicht –
        Er schlachte der Opfer zweie 
        Und glaube an Liebe und Treue.“
        
        Und die Sonne geht unter, da steht er am Tor 
        Und sieht das Kreuz schon erhöhet, 
        Das die Menge gaffend umstehet; 
        An dem Seile schon zieht man den Freund empor, 
        Da zertrennt er gewaltig den dichten Chor: 
        „Mich, Henker!“ ruft er, „erwürget!
        Da bin ich, für den er gebürget!“
        
        Und Erstaunen ergreifet das Volk umher, 
        In den Armen liegen sich beide, 
        Und weinen für Schmerzen und Freude. 
        Da sieht man kein Auge tränenleer, 
        Und zum Könige bringt man die Wundermär, 
        Der fühlt ein menschliches Rühren, 
        Läßt schnell vor den Thron sie führen.
        
        Und blicket sie lange verwundert an, 
        Drauf spricht er: „Es ist euch gelungen, 
        Ihr habt das Herz mir bezwungen, 
        Und die Treue, sie ist doch kein leerer Wahn – 
        So nehmet auch mich zum Genossen an, 
        Ich sei, gewährt mir die Bitte, 
        In eurem Bunde der Dritte.“
        
        Und die Freunde umarmen den König gleich,
        Und sie nennen sich Brüder und schwören
        Auf ewig sich Treue und Ehren.
        Und der König läßt ihm sein Leben zugleich,
        Und er feieret mit ihnen das Frühlingsfest,
        Und kein Auge bleibt trübe,
        Und die Freude bekämpfet die Liebe.
        ".split_ascii_whitespace().cycle().take(40000).join(" ").into(),
        //text: "Availablitytestability! Yes".into(),
        //text: "This is a little test case".into(),
        font: "Helvetica Neue".to_string(),
        font_size: 12.0,
    };

    /*content.add_element(Box::new(
        QRCodeLayouter {
            content: "https://www.google.com".to_string(),
            size: 100.0,
        }
    ));*/
    //content.add_element(Box::new(text));

    let rows = (0..200).map(|_| {
        RowInfo {
            cells: vec![
                CellInfo {
                    content: Box::new(
                        QRCodeLayouter {
                            content: "https://www.google.com".to_string(),
                            size: 35.0,
                        }
                    ),
                    width: open_tab_reports::layout::design::CellWidth::Fixed(40.0)
                },
                CellInfo { content: Box::new(
                    TextLayouter {
                        text: "Julius Steen\nSchönrederei\nRederei".into(),
                        font: "Helvetica Neue".to_string(),
                        font_size: 12.0,
                    }
                ), width: open_tab_reports::layout::design::CellWidth::Dynamic },
            ]
        }
    }).collect_vec();

    content.add_element(
        Box::new(
            TabularLayouter {
                rows,
                row_margin: 10.0
            }
        )
    );
    let doc = content.layout().unwrap();
    let data = doc.write_as_pdf().unwrap();
    std::fs::write("test-new-2.pdf", data).unwrap();

}