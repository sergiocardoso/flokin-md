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
action-back = Voltar
action-previous = Anterior
action-next = Próxima
action-yes = Sim
action-no = Não
state-on = ON
state-off = OFF

# Menus
menu-file = Arquivo
menu-view = Exibir
menu-navigate = Navegar
menu-data = Dados
menu-help = Ajuda
menu-new-file = Novo arquivo
menu-open-folder = Abrir pasta
menu-close-folder = Fechar pasta
menu-reindex = Reindexar
menu-toggle-theme = Alternar tema
menu-left-sidebar = Barra lateral esquerda
menu-right-sidebar = Barra lateral direita
menu-files = Arquivos
menu-context = Contexto
menu-graph = Grafo
menu-health = Saúde do banco
menu-sql-explorer = SQL Explorer
menu-history = Histórico
menu-settings = Configurações
menu-search = Buscar
menu-open-data = Abrir Dados
menu-open-graph = Abrir Grafo
menu-run-query = Executar query
menu-about = Sobre o FlokinMD

# Activity
activity-files = Arquivos
activity-data = Dados
activity-context = Contexto
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
about-version = Versão { $version }
about-tagline = Um banco de dados local-first para seus arquivos Markdown.
about-description = FlokinMD é um projeto open source criado para transformar pastas Markdown em espaços de dados estruturados, sem tirar do usuário a propriedade dos seus arquivos.
about-source-truth = Markdown continua sendo a fonte de verdade.
about-open-source = Projeto open source
about-created-by = Criado por Sérgio Cardoso.
about-motivation-title = Por que o FlokinMD existe
about-motivation-paragraph-1 = Markdown deixou de ser apenas documentação.
about-motivation-paragraph-2 =
    Com IA, agentes, SDD, skills e workflows orientados a contexto, arquivos .md passaram a funcionar cada vez mais como memória e infraestrutura de projetos.
about-motivation-paragraph-3 =
    Specs, instruções, prompts, decisões, contextos e conhecimento operacional estão vivendo em Markdown.
about-motivation-paragraph-4 =
    O FlokinMD nasceu para tornar essa base fácil de consultar, editar, visualizar e, principalmente, entender como os documentos se relacionam.
about-context-highlight = Markdown virou infraestrutura de contexto.
about-creator-title = Sobre o criador
about-creator-role = Software Engineer · Criador do FlokinMD
about-creator-paragraph-1 =
    Criei o FlokinMD a partir de uma necessidade que comecei a sentir no meu próprio trabalho com software, agentes e IA: quanto mais contexto passava a viver em Markdown, mais difícil ficava enxergar essa base como um sistema.
about-creator-paragraph-2 =
    Queria uma ferramenta rápida e direta para abrir uma pasta, consultar, editar, visualizar e entender os relacionamentos entre os documentos — sem precisar adotar uma suíte completa ou migrar meus arquivos para outro formato.
about-flokin-project = FlokinMD é um projeto open source da Flokin.
about-flokin-project-short = Um projeto Flokin.
about-built-with = Construído com Rust + Iced.
about-linkedin = LinkedIn
about-website = Site
about-email = Email
about-manifesto = Markdown is the database.
about-principle = Markdown is the database.

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
explorer-new-file = Novo arquivo
explorer-file-name = Nome do arquivo
explorer-create-file = Criar arquivo
explorer-file-exists = Já existe um arquivo com esse nome.
explorer-file-create-failed = Não foi possível criar o arquivo: { $message }
explorer-invalid-file-name = Informe um nome de arquivo Markdown válido.
explorer-expand-all = Expandir tudo
explorer-collapse-all = Recolher tudo
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
explorer-database = DATABASE
semantic-agent = Agente
semantic-agent-instructions = Instruções de agente
semantic-skill = Skill
semantic-spec = Spec
semantic-ice = ICE
semantic-context = Contexto
semantic-prompt = Prompt
semantic-rules = Regras
semantic-memory = Memória
semantic-mcp = MCP
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
status-read-only = Somente leitura
status-sqlite-memory = SQLite :memory:
status-markdown = Markdown
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
        [one] 1 aviso
       *[other] { $count } avisos
    }

# Editor
editor-empty-workspace-hint = Abra uma pasta que contenha arquivos .md ou .markdown.
editor-scanned-folder = Pasta escaneada: { $path }
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
sql-query-tab = Consulta 1
sql-review-update = Revisar atualização
sql-run = Executar
sql-mode-query = Consulta
sql-mode-update = Atualização
sql-update-context = SQL Updates são convertidos em alterações Markdown e sempre exigem preview.
sql-query-context = Modo Consulta é read-only.
sql-results = RESULTADOS
sql-error = Erro:
sql-no-results = Sem resultados
sql-preview-status = { $matched } documentos correspondem • { $changed } serão alterados
sql-result-status = { $rows } linhas • { $ms } ms
sql-results-limited = Resultados limitados a 1.000 linhas.
sql-empty-update-preview = Revise uma atualização UPDATE para ver o preview.
sql-empty-query-results = Execute uma consulta SELECT para ver o grid.
sql-update-no-matches = Nenhum documento corresponde a esta atualização.
sql-update-no-changes =
    { $count ->
        [one] 1 documento corresponde, mas nenhuma alteração é necessária.
       *[other] { $count } documentos correspondem, mas nenhuma alteração é necessária.
    }
sql-documents-match =
    { $count ->
        [one] 1 documento corresponde
       *[other] { $count } documentos correspondem
    }
sql-no-result-columns = Consulta executada sem colunas de resultado.
sql-schema-table-name = SQL: { $table }
sql-column-type-text = TEXT
sql-column-type-integer = INTEGER
sql-column-type-real = REAL
sql-column-type-boolean = BOOLEAN
sql-column-type-json = JSON
sql-column-type-null = NULL

# Data / Schema / Bulk
data-properties =
    { $count ->
        [one] 1 propriedade
       *[other] { $count } propriedades
    }
data-panel-data = Dados
data-panel-schema = Schema
data-empty-collection = Nenhum documento nesta Collection.
data-empty-schema = Nenhum schema disponível para esta Collection.
schema-source-inferred = Schema inferido
schema-source-explicit = Schema explícito + observações inferidas
schema-inferred-title = Schema inferido
schema-inferred-description = O FlokinMD detectou esta estrutura automaticamente a partir dos seus documentos. Crie um schema explícito para definir tipos e campos obrigatórios.
schema-explicit-title = Schema explícito
schema-explicit-invalid = Schema explícito inválido
schema-field = FIELD
schema-type = TYPE
schema-required = REQUIRED
schema-present = PRESENT
schema-present-in = Present in
schema-present-ratio = { $observed } / { $total } documentos
schema-null-values = Null values
schema-declared = Declared
schema-not-declared = Não declarado
schema-observed-types = Observed types: { $types }
schema-unknown = Unknown
schema-structural-suffix =  · campo estrutural/derivado
bulk-edit-title = Editar em massa
bulk-clear-selection = Limpar seleção
bulk-selected-count =
    { $count ->
        [one] 1 selecionado
       *[other] { $count } selecionados
    }
bulk-selected-documents =
    { $count ->
        [one] 1 documento selecionado
       *[other] { $count } documentos selecionados
    }
bulk-step-configure = 1. Configurar
bulk-step-review = 2. Revisar
bulk-preview-unavailable = Preview indisponível. Volte e revise a configuração.
bulk-review-changes = Revisar alterações
bulk-operation = Operação
bulk-operation-set = Definir propriedade
bulk-operation-remove = Remover propriedade
bulk-new-property-option = + Nova propriedade...
bulk-property-placeholder = Escolha uma propriedade
bulk-property = Propriedade
bulk-property-name = Nome da propriedade
bulk-property-name-placeholder = ex.: reviewed
bulk-type = Tipo
bulk-value = Valor
bulk-target = Destino
bulk-target-placeholder = Destino
bulk-value-placeholder = Valor
bulk-null-value = Valor: null
value-true = Verdadeiro
value-false = Falso
value-type-string = Texto
value-type-integer = Inteiro
value-type-float = Decimal
value-type-boolean = Booleano
value-type-array = Lista
value-type-object = Objeto
value-type-mixed = Mixed
value-type-null = Nulo
value-type-relation = Relação
change-status-changed = Alterado
change-status-no-change = Sem alteração
change-status-blocked = Bloqueado
change-status-unsupported = Não suportado
apply-changes =
    { $count ->
        [one] Aplicar 1 alteração
       *[other] Aplicar { $count } alterações
    }
changes-will-change =
    { $count ->
        [one] 1 será alterado
       *[other] { $count } serão alterados
    }
changes-no-change =
    { $count ->
        [one] 1 sem alteração
       *[other] { $count } sem alteração
    }
changes-blocked =
    { $count ->
        [one] 1 bloqueado
       *[other] { $count } bloqueados
    }
pagination-status = { $start }-{ $end } de { $total }  ·  Página { $page } de { $pages }

# Settings
settings-section-interface = INTERFACE
settings-language = Idioma
settings-section-appearance = APARÊNCIA
settings-theme = Tema
theme-light = Claro
theme-dark = Escuro
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
health-expected = EXPECTED
health-found = FOUND
health-workspace = workspace
health-severity-error = Erro
health-severity-warning = Warning
health-severity-info = Info
health-category-parsing = Parsing
health-category-schema = Schema
health-category-relations = Relations
health-category-workspace = Workspace
health-issue-invalid-frontmatter = Frontmatter YAML inválido.
health-issue-file-read-error = Não foi possível ler o arquivo.
health-issue-workspace-scan-error = Erro ao processar o workspace.
health-issue-explicit-schema-invalid = Schema explícito inválido.
health-issue-required-field-missing = Campo obrigatório ausente.
health-issue-type-mismatch = Esperado { $expected }, encontrado { $found }.
health-issue-undeclared-field = Campo não declarado no schema explícito.
health-issue-mixed-observed-types = Tipos inconsistentes observados.
health-issue-relation-unresolved = Relação não resolvida.
health-issue-relation-ambiguous =
    { $count ->
        [one] Relação ambígua: 1 documento corresponde.
       *[other] Relação ambígua: { $count } documentos correspondem.
    }

# Inspector
inspector-properties = PROPRIEDADES
inspector-relations = RELAÇÕES
inspector-referenced-by = REFERENCIADO POR
inspector-tags = TAGS
inspector-warnings = WARNINGS
inspector-metadata = METADADOS
inspector-issue = ISSUE
inspector-details = DETALHES
inspector-open-document = Abrir documento
relation-unresolved = Não resolvido
relation-ambiguous-count =
    { $count ->
        [one] Ambíguo — 1 documento corresponde
       *[other] Ambíguo — { $count } documentos correspondem
    }
relation-structured-reference = referência estruturada

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
graph-fit = Ajustar à tela

# Context
context-title = Contexto
context-sidebar-title = CONTEXTO
context-overview = Visão geral
context-agents = Agentes
context-skills = Skills
context-specs = SDD / Specs
context-ice = ICE
context-contexts = Contextos
context-prompts = Prompts
context-rules = Regras / Instruções
context-memory = Memória
context-mcp = MCP
context-artifact-count =
    { $count ->
        [one] 1 artefato
       *[other] { $count } artefatos
    }
context-artifacts = ARTEFATOS
context-name = Nome
context-kind = Tipo
context-path = Path
context-relations = Relações
context-referenced-by = Referenciado por
context-references = Referencia
context-unconnected = Sem relações
context-unconnected-count =
    { $count ->
        [one] 1 artefato
       *[other] { $count } artefatos
    }
context-inspector-title = CONTEXTO
context-select-artifact = Selecione um artefato para ver detalhes.
context-open-editor = Abrir no editor
context-show-graph = Mostrar no grafo
context-metadata = Metadados
context-unresolved = não resolvido
context-ambiguous = ambíguo
context-empty = Nenhum artefato de contexto encontrado neste workspace.
context-no-agents = Nenhum agente encontrado neste workspace.
context-no-skills = Nenhuma Skill encontrada neste workspace.
context-no-specs = Nenhuma Spec encontrada neste workspace.
context-no-ice = Nenhum ICE encontrado neste workspace.
context-no-contexts = Nenhum contexto encontrado neste workspace.
context-no-prompts = Nenhum prompt encontrado neste workspace.
context-no-rules = Nenhuma regra ou instrução encontrada neste workspace.
context-no-memory = Nenhuma memória encontrada neste workspace.
context-no-mcp = Nenhum MCP encontrado neste workspace.

# Welcome
welcome-title = Abra uma pasta Markdown para começar
welcome-open-folder = Abrir pasta
workspace-restoring = Abrindo workspace...
workspace-previous-unavailable = A pasta usada anteriormente não está disponível.
