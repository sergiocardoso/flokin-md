# Common
app-name = FlokinMD
action-apply = Aplicar
action-cancel = Cancelar
action-close = Fechar
action-discard = Descartar alterações
action-open = Abrir
action-save = Salvar
action-save-all = Salvar tudo
action-do-not-save = Não salvar
action-reset = Restaurar
state-on = ON
state-off = OFF

# Menus
menu-file = Arquivo
menu-view = Exibir
menu-navigate = Navegar
menu-data = Dados
menu-help = Ajuda
menu-open-folder = Abrir pasta
menu-reindex = Reindexar
menu-toggle-theme = Alternar tema
menu-left-sidebar = Barra lateral esquerda
menu-right-sidebar = Barra lateral direita
menu-files = Arquivos
menu-graph = Grafo
menu-health = Saúde do banco
menu-sql-explorer = SQL Explorer
menu-history = Histórico
menu-settings = Configurações
menu-search = Buscar
menu-open-data = Abrir Dados
menu-open-graph = Abrir Grafo
menu-run-query = Executar query
menu-about = Sobre FlokinMD

# Activity
activity-files = Arquivos
activity-data = Dados
activity-graph = Grafo
activity-health = Saúde do banco
activity-sql = SQL Explorer
activity-history = Histórico
activity-settings = Configurações

# Top Shell
search-placeholder = Buscar documentos...
tooltip-hide-left-sidebar = Ocultar barra lateral esquerda
tooltip-show-left-sidebar = Mostrar barra lateral esquerda
tooltip-hide-right-sidebar = Ocultar barra lateral direita
tooltip-show-right-sidebar = Mostrar barra lateral direita
tooltip-toggle-theme = Alternar tema
about-description = Workspace Markdown com projeção SQL descartável.

# Search
search-type-to-search = Digite para buscar documentos.
search-no-results = Nenhum documento encontrado para "{ $query }".
search-results =
    { $count ->
        [one] 1 resultado
       *[other] { $count } resultados
    }
search-results-limited = { $count }+ resultados

# Settings
settings-section-interface = INTERFACE
settings-language = Idioma
settings-section-appearance = APARÊNCIA
settings-theme = Tema
settings-section-layout = LAYOUT
settings-hide-left-sidebar = Ocultar barra lateral esquerda
settings-show-left-sidebar = Mostrar barra lateral esquerda
settings-hide-right-sidebar = Ocultar barra lateral direita
settings-show-right-sidebar = Mostrar barra lateral direita
settings-reset-layout = Restaurar layout padrão

# Dialogs
dirty-close-title = Salvar alterações em { $file }?
dirty-close-description = Suas alterações ainda não foram gravadas no Markdown.
dirty-workspace-title =
    { $count ->
        [one] Existe 1 arquivo com alterações não salvas.
       *[other] Existem { $count } arquivos com alterações não salvas.
    }
dirty-workspace-description = Escolha como lidar com as tabs sujas antes de continuar.
schema-create-title = Criar schema explícito
schema-create-description = O FlokinMD criará { $file } na raiz deste workspace.
schema-exists-warning = Já existe um flokin.schema.yaml neste workspace.
schema-none-available = Nenhuma Collection disponível para gerar schema.
schema-from-inferred = O arquivo será gerado a partir do Schema atualmente inferido.
schema-detected-collections = Collections detectadas:
schema-collection-count =
    { $count ->
        [one] { $name } (1 documento)
       *[other] { $name } ({ $count } documentos)
    }
schema-mixed-fields-omitted = Campos Mixed serão omitidos: { $fields }.
schema-open = Abrir schema
schema-create = Criar schema

# Feedback
files-restored =
    { $count ->
        [one] 1 arquivo restaurado.
       *[other] { $count } arquivos restaurados.
    }
error-no-workspace-schema = Nenhum workspace aberto.
error-schema-empty = Nenhuma Collection disponível para gerar schema.
error-schema-generate = Não foi possível gerar o schema: { $error }
error-stale-preview = O workspace mudou desde a geração do preview. Revise as alterações novamente.
error-save-file = Não foi possível salvar { $path }: { $error }
error-history-clear-no-workspace = Abra um workspace para limpar o histórico.
