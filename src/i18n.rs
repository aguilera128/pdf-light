//! Textos de la interfaz. El idioma sale del de Windows (o de `Language` en settings.json);
//! si no está traducido se usa el inglés.

use std::sync::OnceLock;
use windows_sys::Win32::Globalization::GetUserDefaultUILanguage;

pub struct Strings {
    pub intro: &'static str,
    pub name: &'static str,
    pub folder: &'static str,
    pub browse: &'static str,
    pub quality: &'static str,
    pub open_after: &'static str,
    pub auto_save: &'static str,
    pub discard: &'static str,
    pub save: &'static str,
    pub saving: &'static str,
    /// `{}` = número de trabajos.
    pub merge_save: &'static str,
    pub untitled: &'static str,
    pub default_name: &'static str,
    /// En el orden de `logic::QUALITIES`.
    pub qualities: [&'static str; 5],
    pub tray_pending: &'static str,
    pub tray_open_folder: &'static str,
    pub tray_auto: &'static str,
    pub tray_exit: &'static str,
    pub saved: &'static str,
    pub auto_failed: &'static str,
    pub create_failed: &'static str,
    pub invalid_name: &'static str,
    pub invalid_folder: &'static str,
    /// `{}` = nombre del fichero.
    pub exists: &'static str,
    pub discard_one: &'static str,
    /// `{}` = número de trabajos.
    pub discard_many: &'static str,
    pub browse_title: &'static str,
    pub err_no_gs: &'static str,
    /// `{}` = error del sistema.
    pub err_launch: &'static str,
    /// `{}` = estado de salida, `{}` = salida de Ghostscript.
    pub err_gs_failed: &'static str,
    /// `{}` = error del sistema.
    pub err_mkdir: &'static str,
    /// `{}` = ruta, `{}` = error del sistema.
    pub err_write: &'static str,
}

/// Sustituye cada `{}` de `template` por el argumento que toca.
pub fn fill(template: &str, args: &[&str]) -> String {
    let mut out = String::new();
    let mut parts = template.split("{}");
    out.push_str(parts.next().unwrap_or(""));
    for (i, part) in parts.enumerate() {
        out.push_str(args.get(i).copied().unwrap_or(""));
        out.push_str(part);
    }
    out
}

static CURRENT: OnceLock<&'static Strings> = OnceLock::new();

/// Fija el idioma. `code` es el de settings.json ("es", "en"...); `None` = el de Windows.
pub fn init(code: Option<&str>) {
    let lang = code.and_then(by_code).unwrap_or_else(|| {
        // Identificador primario de idioma: los 10 bits bajos del LANGID.
        match unsafe { GetUserDefaultUILanguage() } & 0x3FF {
            0x0A => &ES,
            0x0C => &FR,
            0x07 => &DE,
            0x16 => &PT,
            0x10 => &IT,
            _ => &EN,
        }
    });
    let _ = CURRENT.set(lang);
}

fn by_code(code: &str) -> Option<&'static Strings> {
    Some(match code.to_ascii_lowercase().get(..2)? {
        "en" => &EN,
        "es" => &ES,
        "fr" => &FR,
        "de" => &DE,
        "pt" => &PT,
        "it" => &IT,
        _ => return None,
    })
}

pub fn t() -> &'static Strings {
    CURRENT.get().copied().unwrap_or(&EN)
}

static EN: Strings = Strings {
    intro: "Pending jobs. Select several (Ctrl or Shift) to merge them into a single PDF.",
    name: "&Name",
    folder: "&Folder",
    browse: "&Browse...",
    quality: "&Quality",
    open_after: "&Open the PDF when done",
    auto_save: "Save &without asking from now on (to this folder)",
    discard: "&Discard",
    save: "Save",
    saving: "Saving...",
    merge_save: "Merge and save ({})",
    untitled: "(untitled)",
    default_name: "Document",
    qualities: [
        "Screen (72 dpi, smallest size)",
        "Standard (150 dpi)",
        "High (300 dpi)",
        "Prepress (300 dpi, colour-accurate)",
        "Do not recompress images",
    ],
    tray_pending: "Pending jobs",
    tray_open_folder: "Open output folder",
    tray_auto: "Save without asking",
    tray_exit: "Exit",
    saved: "PDF saved",
    auto_failed: "Could not save automatically",
    create_failed: "The PDF could not be created.",
    invalid_name: "The name is empty or contains invalid characters (\\ / : * ? \" < > |).",
    invalid_folder: "Enter a folder with its full path.",
    exists: "\u{201C}{}\u{201D} already exists. Replace it?",
    discard_one: "Discard the selected job? This cannot be undone.",
    discard_many: "Discard {} jobs? This cannot be undone.",
    browse_title: "Folder to save PDFs in",
    err_no_gs: "Ghostscript (gswin64c.exe) was not found. Install it or set its path in GhostscriptPath in %AppData%\\PDFLight\\settings.json.",
    err_launch: "Could not run Ghostscript: {}",
    err_gs_failed: "Ghostscript failed ({}).\n{}",
    err_mkdir: "Could not create the folder: {}",
    err_write: "Could not write \u{201C}{}\u{201D}: {}",
};

static ES: Strings = Strings {
    intro: "Trabajos pendientes. Selecciona varios (Ctrl o Mayús) para unirlos en un solo PDF.",
    name: "&Nombre",
    folder: "&Carpeta",
    browse: "E&xaminar...",
    quality: "C&alidad",
    open_after: "A&brir el PDF al terminar",
    auto_save: "&Guardar sin preguntar a partir de ahora (en esta carpeta)",
    discard: "&Descartar",
    save: "Guardar",
    saving: "Guardando...",
    merge_save: "Unir y guardar ({})",
    untitled: "(sin título)",
    default_name: "Documento",
    qualities: [
        "Pantalla (72 ppp, mínimo tamaño)",
        "Estándar (150 ppp)",
        "Alta (300 ppp)",
        "Imprenta (300 ppp, color fiel)",
        "Sin recomprimir imágenes",
    ],
    tray_pending: "Trabajos pendientes",
    tray_open_folder: "Abrir carpeta de salida",
    tray_auto: "Guardar sin preguntar",
    tray_exit: "Salir",
    saved: "PDF guardado",
    auto_failed: "No se pudo guardar automáticamente",
    create_failed: "No se pudo crear el PDF.",
    invalid_name: "El nombre está vacío o tiene caracteres no válidos (\\ / : * ? \" < > |).",
    invalid_folder: "Indica una carpeta con ruta completa.",
    exists: "«{}» ya existe. ¿Sustituirlo?",
    discard_one: "¿Descartar el trabajo seleccionado? No se puede deshacer.",
    discard_many: "¿Descartar {} trabajos? No se puede deshacer.",
    browse_title: "Carpeta donde guardar los PDF",
    err_no_gs: "No se encuentra Ghostscript (gswin64c.exe). Instálalo o indica su ruta en GhostscriptPath de %AppData%\\PDFLight\\settings.json.",
    err_launch: "No se pudo ejecutar Ghostscript: {}",
    err_gs_failed: "Ghostscript falló ({}).\n{}",
    err_mkdir: "No se pudo crear la carpeta: {}",
    err_write: "No se pudo escribir «{}»: {}",
};

static FR: Strings = Strings {
    intro: "Travaux en attente. Sélectionnez-en plusieurs (Ctrl ou Maj) pour les fusionner en un seul PDF.",
    name: "&Nom",
    folder: "&Dossier",
    browse: "&Parcourir...",
    quality: "&Qualité",
    open_after: "&Ouvrir le PDF une fois terminé",
    auto_save: "&Enregistrer sans demander désormais (dans ce dossier)",
    discard: "&Supprimer",
    save: "Enregistrer",
    saving: "Enregistrement...",
    merge_save: "Fusionner ({})",
    untitled: "(sans titre)",
    default_name: "Document",
    qualities: [
        "Écran (72 ppp, taille minimale)",
        "Standard (150 ppp)",
        "Haute (300 ppp)",
        "Prépresse (300 ppp, couleurs fidèles)",
        "Ne pas recompresser les images",
    ],
    tray_pending: "Travaux en attente",
    tray_open_folder: "Ouvrir le dossier de sortie",
    tray_auto: "Enregistrer sans demander",
    tray_exit: "Quitter",
    saved: "PDF enregistré",
    auto_failed: "Échec de l'enregistrement automatique",
    create_failed: "Impossible de créer le PDF.",
    invalid_name: "Le nom est vide ou contient des caractères non valides (\\ / : * ? \" < > |).",
    invalid_folder: "Indiquez un dossier avec son chemin complet.",
    exists: "« {} » existe déjà. Le remplacer ?",
    discard_one: "Supprimer le travail sélectionné ? Cette action est irréversible.",
    discard_many: "Supprimer {} travaux ? Cette action est irréversible.",
    browse_title: "Dossier où enregistrer les PDF",
    err_no_gs: "Ghostscript (gswin64c.exe) est introuvable. Installez-le ou indiquez son chemin dans GhostscriptPath de %AppData%\\PDFLight\\settings.json.",
    err_launch: "Impossible d'exécuter Ghostscript : {}",
    err_gs_failed: "Ghostscript a échoué ({}).\n{}",
    err_mkdir: "Impossible de créer le dossier : {}",
    err_write: "Impossible d'écrire « {} » : {}",
};

static DE: Strings = Strings {
    intro: "Ausstehende Aufträge. Mehrere auswählen (Strg oder Umschalt), um sie zu einem PDF zusammenzuführen.",
    name: "&Name",
    folder: "&Ordner",
    browse: "&Durchsuchen...",
    quality: "&Qualität",
    open_after: "PDF nach dem Speichern ö&ffnen",
    auto_save: "Ab jetzt ohne Nachfrage &speichern (in diesem Ordner)",
    discard: "&Verwerfen",
    save: "Speichern",
    saving: "Speichern...",
    merge_save: "Zusammenführen ({})",
    untitled: "(ohne Titel)",
    default_name: "Dokument",
    qualities: [
        "Bildschirm (72 dpi, kleinste Datei)",
        "Standard (150 dpi)",
        "Hoch (300 dpi)",
        "Druckvorstufe (300 dpi, farbtreu)",
        "Bilder nicht neu komprimieren",
    ],
    tray_pending: "Ausstehende Aufträge",
    tray_open_folder: "Ausgabeordner öffnen",
    tray_auto: "Ohne Nachfrage speichern",
    tray_exit: "Beenden",
    saved: "PDF gespeichert",
    auto_failed: "Automatisches Speichern fehlgeschlagen",
    create_failed: "Das PDF konnte nicht erstellt werden.",
    invalid_name: "Der Name ist leer oder enthält ungültige Zeichen (\\ / : * ? \" < > |).",
    invalid_folder: "Geben Sie einen Ordner mit vollständigem Pfad an.",
    exists: "\u{201E}{}\u{201C} ist bereits vorhanden. Ersetzen?",
    discard_one: "Ausgewählten Auftrag verwerfen? Dies kann nicht rückgängig gemacht werden.",
    discard_many: "{} Aufträge verwerfen? Dies kann nicht rückgängig gemacht werden.",
    browse_title: "Ordner zum Speichern der PDFs",
    err_no_gs: "Ghostscript (gswin64c.exe) wurde nicht gefunden. Installieren Sie es oder geben Sie den Pfad unter GhostscriptPath in %AppData%\\PDFLight\\settings.json an.",
    err_launch: "Ghostscript konnte nicht gestartet werden: {}",
    err_gs_failed: "Ghostscript ist fehlgeschlagen ({}).\n{}",
    err_mkdir: "Der Ordner konnte nicht erstellt werden: {}",
    err_write: "\u{201E}{}\u{201C} konnte nicht geschrieben werden: {}",
};

// Portugués de Brasil.
static PT: Strings = Strings {
    intro: "Trabalhos pendentes. Selecione vários (Ctrl ou Shift) para uni-los em um único PDF.",
    name: "&Nome",
    folder: "&Pasta",
    browse: "P&rocurar...",
    quality: "&Qualidade",
    open_after: "&Abrir o PDF ao terminar",
    auto_save: "&Salvar sem perguntar a partir de agora (nesta pasta)",
    discard: "&Descartar",
    save: "Salvar",
    saving: "Salvando...",
    merge_save: "Unir e salvar ({})",
    untitled: "(sem título)",
    default_name: "Documento",
    qualities: [
        "Tela (72 dpi, tamanho mínimo)",
        "Padrão (150 dpi)",
        "Alta (300 dpi)",
        "Pré-impressão (300 dpi, cores fiéis)",
        "Não recomprimir imagens",
    ],
    tray_pending: "Trabalhos pendentes",
    tray_open_folder: "Abrir pasta de saída",
    tray_auto: "Salvar sem perguntar",
    tray_exit: "Sair",
    saved: "PDF salvo",
    auto_failed: "Não foi possível salvar automaticamente",
    create_failed: "Não foi possível criar o PDF.",
    invalid_name: "O nome está vazio ou contém caracteres inválidos (\\ / : * ? \" < > |).",
    invalid_folder: "Informe uma pasta com o caminho completo.",
    exists: "\u{201C}{}\u{201D} já existe. Substituir?",
    discard_one: "Descartar o trabalho selecionado? Esta ação não pode ser desfeita.",
    discard_many: "Descartar {} trabalhos? Esta ação não pode ser desfeita.",
    browse_title: "Pasta onde salvar os PDFs",
    err_no_gs: "O Ghostscript (gswin64c.exe) não foi encontrado. Instale-o ou informe o caminho em GhostscriptPath de %AppData%\\PDFLight\\settings.json.",
    err_launch: "Não foi possível executar o Ghostscript: {}",
    err_gs_failed: "O Ghostscript falhou ({}).\n{}",
    err_mkdir: "Não foi possível criar a pasta: {}",
    err_write: "Não foi possível gravar \u{201C}{}\u{201D}: {}",
};

static IT: Strings = Strings {
    intro: "Lavori in attesa. Selezionane più di uno (Ctrl o Maiusc) per unirli in un unico PDF.",
    name: "&Nome",
    folder: "&Cartella",
    browse: "&Sfoglia...",
    quality: "&Qualità",
    open_after: "&Apri il PDF al termine",
    auto_save: "Sal&va senza chiedere d'ora in poi (in questa cartella)",
    discard: "&Elimina",
    save: "Salva",
    saving: "Salvataggio...",
    merge_save: "Unisci e salva ({})",
    untitled: "(senza titolo)",
    default_name: "Documento",
    qualities: [
        "Schermo (72 dpi, dimensione minima)",
        "Standard (150 dpi)",
        "Alta (300 dpi)",
        "Prestampa (300 dpi, colori fedeli)",
        "Non ricomprimere le immagini",
    ],
    tray_pending: "Lavori in attesa",
    tray_open_folder: "Apri cartella di destinazione",
    tray_auto: "Salva senza chiedere",
    tray_exit: "Esci",
    saved: "PDF salvato",
    auto_failed: "Impossibile salvare automaticamente",
    create_failed: "Impossibile creare il PDF.",
    invalid_name: "Il nome è vuoto o contiene caratteri non validi (\\ / : * ? \" < > |).",
    invalid_folder: "Indica una cartella con il percorso completo.",
    exists: "«{}» esiste già. Sostituirlo?",
    discard_one: "Eliminare il lavoro selezionato? L'operazione non può essere annullata.",
    discard_many: "Eliminare {} lavori? L'operazione non può essere annullata.",
    browse_title: "Cartella in cui salvare i PDF",
    err_no_gs: "Ghostscript (gswin64c.exe) non è stato trovato. Installalo o indica il percorso in GhostscriptPath di %AppData%\\PDFLight\\settings.json.",
    err_launch: "Impossibile eseguire Ghostscript: {}",
    err_gs_failed: "Ghostscript non è riuscito ({}).\n{}",
    err_mkdir: "Impossibile creare la cartella: {}",
    err_write: "Impossibile scrivere «{}»: {}",
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fill_placeholders() {
        assert_eq!(fill("a {} b {} c", &["1", "2"]), "a 1 b 2 c");
        assert_eq!(fill("sin huecos", &["x"]), "sin huecos");
        assert_eq!(fill("{} solo", &[]), " solo");
    }

    #[test]
    fn every_language_keeps_its_placeholders() {
        for l in [&EN, &ES, &FR, &DE, &PT, &IT] {
            assert_eq!(l.merge_save.matches("{}").count(), 1);
            assert_eq!(l.exists.matches("{}").count(), 1);
            assert_eq!(l.discard_many.matches("{}").count(), 1);
            assert_eq!(l.err_launch.matches("{}").count(), 1);
            assert_eq!(l.err_gs_failed.matches("{}").count(), 2);
            assert_eq!(l.err_mkdir.matches("{}").count(), 1);
            assert_eq!(l.err_write.matches("{}").count(), 2);
        }
    }
}
