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

# Explorer
explorer-title = EXPLORADOR
explorer-open = Abrir
explorer-reindex = Reindexar
explorer-no-workspace = Nenhuma pasta aberta
explorer-scanning = Analisando documentos...
explorer-scan-failed = Falha ao analisar workspace
explorer-no-markdown = Nenhum arquivo Markdown encontrado.
explorer-files = FILES
explorer-data = DATA
explorer-collections = COLLECTIONS
explorer-filters = FILTROS
explorer-filters-empty = Disponíveis após indexação
explorer-documents-found =
    { $count ->
        [one] 1 documento encontrado
       *[other] { $count } documentos encontrados
    }
explorer-access-errors =
    { $count ->
        [one] 1 item não pôde ser acessado
       *[other] { $count } itens não puderam ser acessados
    }
explorer-warnings =
    { $count ->
        [one] 1 warning
       *[other] { $count } warnings
    }
sql-schema-empty = Nenhuma Collection disponível.
sql-schema-building = Construindo schema...
sql-schema-open-folder = Abra uma pasta para gerar o schema.

# Status Bar
status-no-workspace = Nenhuma pasta aberta
status-scanning = Analisando documentos...
status-updating = Atualizando...
status-scan-failed = Falha ao analisar workspace
status-workspace-watched = Workspace monitorado
status-read-only = Read only
status-documents =
    { $count ->
        [one] 1 documento
       *[other] { $count } documentos
    }
status-collections =
    { $count ->
        [one] 1 collection
       *[other] { $count } collections
    }
status-warnings =
    { $count ->
        [one] 1 warning
       *[other] { $count } warnings
    }

# Editor
editor-empty-workspace-hint = Abra uma pasta que contenha arquivos .md ou .markdown.
editor-mode-edit = Editar
editor-mode-split = Dividido
editor-mode-preview = Prévia
editor-save-tooltip = Salvar (Ctrl+S)
editor-conflict-modified = O arquivo foi alterado externamente.
editor-conflict-deleted = O arquivo foi removido externamente.
editor-reload-disk = Recarregar do disco
editor-keep-local = Manter minhas alterações
editor-empty-preview = Prévia vazia.
editor-empty-file = Arquivo vazio.
editor-select-document = Selecione um documento Markdown para ver o conteúdo.

# SQL
sql-reviewing = Revisando...
sql-running = Executando...
sql-review-update = Revisar atualização
sql-run = Executar
sql-mode-query = Consulta
sql-mode-update = Atualização
sql-update-context = SQL Updates são convertidos em alterações Markdown e sempre exigem preview.
sql-query-context = Modo Consulta é read-only.

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

# History
history-title = Histórico
history-description = Operações de Bulk Edit, SQL Update e Undo registradas localmente.
history-no-workspace-title = Abra um workspace para ver o histórico.
history-no-workspace-subtitle = O histórico é local e isolado por workspace.
history-empty-title = Nenhuma alteração registrada ainda.
history-empty-subtitle = Operações de Bulk Edit e SQL Update aparecerão aqui.
history-select-title = Selecione uma operação.
history-select-subtitle = Os detalhes aparecerão aqui.
history-clear = Limpar histórico
history-clear-confirm-title = Limpar histórico
history-clear-confirm-description = Isso removerá o histórico de operações deste workspace. Os arquivos Markdown não serão alterados.
history-today = Hoje
history-yesterday = Ontem
history-source-bulk = Bulk Edit
history-source-sql = SQL Update
history-source-undo = Undo
history-undo-available = undo disponível
history-undo-unavailable = undo indisponível
history-undone = desfeita
history-undo-button = Desfazer alteração
history-original-operation = Operação original: { $id }
history-full-content-recorded = Conteúdo completo registrado.
history-full-content-restore = Conteúdo completo será restaurado.
undo-review-title = Revisar desfazer
undo-files-restore =
    { $count ->
        [one] 1 arquivo será restaurado
       *[other] { $count } arquivos serão restaurados
    }
undo-apply =
    { $count ->
        [one] Desfazer 1 alteração
       *[other] Desfazer { $count } alterações
    }

# Health
health-title = Database Health
health-total-documents =
    { $count ->
        [one] 1 documento
       *[other] { $count } documentos
    }
health-errors = Errors
health-warnings = Warnings
health-healthy = Healthy
health-filter-all = All
health-filter-errors = Errors
health-filter-warnings = Warnings
health-filter-placeholder = Filtrar issues...
health-schema-absent = Schema explícito não configurado.
health-schema-inferred = O banco está usando somente a estrutura inferida.
health-schema-invalid = Há um problema em flokin.schema.yaml.
health-no-issues = Nenhuma issue encontrada.
health-severity = SEVERITY
health-category = CATEGORY
health-document = DOCUMENT
health-property = PROPERTY
health-problem = PROBLEM

# Graph
graph-title = Grafo
graph-section = GRAPH
graph-documents = Documents
graph-relations = Relations
graph-problems = Problems
graph-summary = { $documents } documentos • { $relations } relações
graph-zoom-out = Diminuir zoom
graph-zoom-in = Aumentar zoom
graph-zoom-reset = Resetar zoom
graph-focus-selected = Centralizar selecionado
graph-fit = Enquadrar grafo
