import {
  BriefcaseBusiness,
  CalendarDays,
  CheckSquare,
  Cog,
  Database,
  FilePlus2,
  Filter,
  Folder,
  FolderKanban,
  GitPullRequestArrow,
  Home,
  Landmark,
  Search,
  Users,
} from "lucide-react";
import type { ComponentType, SVGProps } from "react";
import "./App.css";

const banks = [
  { name: "Trabalho", icon: BriefcaseBusiness },
  { name: "Pessoal", icon: Home },
];

const collections = [
  { name: "Projetos", icon: FolderKanban },
  { name: "Pessoas", icon: Users },
  { name: "Reuniões", icon: CalendarDays },
  { name: "Tarefas", icon: CheckSquare },
  { name: "Decisões", icon: GitPullRequestArrow },
];

const recentFolders = [
  {
    path: "~/Documents/Knowledge",
    documents: "128 documentos",
    usedAt: "Hoje",
  },
  {
    path: "~/Jobs/JOTA/docs",
    documents: "84 documentos",
    usedAt: "Ontem",
  },
  {
    path: "~/Projects/healthy-chew/docs",
    documents: "46 documentos",
    usedAt: "Semana passada",
  },
];

const benefits = [
  {
    title: "Local-first",
    text: "Seus arquivos ficam onde você escolhe. Funciona offline e sem depender da nuvem.",
  },
  {
    title: "Markdown como fonte de verdade",
    text: "Arquivos abertos, legíveis e portáteis.",
  },
  {
    title: "Sem lock-in",
    text: "Seus dados continuam sendo seus arquivos.",
  },
];

function App() {
  return (
    <div className="app-shell">
      <TopBar />
      <div className="workspace">
        <Sidebar />
        <main className="main-panel">
          <section className="hero-section" aria-labelledby="hero-title">
            <div className="hero-copy">
              <h1 id="hero-title">
                Transforme seus arquivos Markdown em um <span>database visual.</span>
              </h1>
              <p>
                Organize, consulte e relacione seus arquivos .md sem tirá-los do seu
                computador.
              </p>
              <div className="hero-actions" aria-label="Ações iniciais">
                <button className="button button-primary" type="button">
                  <Folder aria-hidden="true" />
                  Abrir pasta
                </button>
                <button className="button button-secondary" type="button">
                  <Database aria-hidden="true" />
                  Criar database vazio
                </button>
              </div>
            </div>
            <InfoPanel />
          </section>
          <RecentFolders />
        </main>
      </div>
      <StatusBar />
    </div>
  );
}

function TopBar() {
  return (
    <header className="top-bar">
      <div className="brand" aria-label="FlokinMD">
        <div className="brand-mark" aria-hidden="true">
          <Landmark />
        </div>
        <span>FlokinMD</span>
      </div>

      <label className="search-box" aria-label="Buscar documentos">
        <Search aria-hidden="true" />
        <input type="search" placeholder="Buscar documentos..." />
        <kbd>Ctrl+K</kbd>
      </label>

      <div className="top-actions">
        <button className="icon-button" type="button" aria-label="Filtros">
          <Filter aria-hidden="true" />
        </button>
        <button className="icon-button" type="button" aria-label="Configurações">
          <Cog aria-hidden="true" />
        </button>
        <button className="button button-primary new-document" type="button">
          <FilePlus2 aria-hidden="true" />
          Novo documento
        </button>
      </div>
    </header>
  );
}

function Sidebar() {
  return (
    <aside className="sidebar" aria-label="Navegação principal">
      <NavGroup title="BANCOS" items={banks} />
      <NavGroup title="COLEÇÕES" items={collections} />
    </aside>
  );
}

type NavItem = {
  name: string;
  icon: ComponentType<SVGProps<SVGSVGElement>>;
};

function NavGroup({ title, items }: { title: string; items: NavItem[] }) {
  return (
    <nav className="nav-group" aria-labelledby={`nav-${title}`}>
      <h2 id={`nav-${title}`}>{title}</h2>
      <ul>
        {items.map((item) => {
          const Icon = item.icon;

          return (
            <li key={item.name}>
              <button className="nav-item" type="button">
                <Icon aria-hidden="true" />
                <span>{item.name}</span>
              </button>
            </li>
          );
        })}
      </ul>
    </nav>
  );
}

function RecentFolders() {
  return (
    <section className="recent-section" aria-labelledby="recent-title">
      <div className="section-heading">
        <h2 id="recent-title">Pastas recentes</h2>
      </div>
      <div className="recent-list">
        {recentFolders.map((folder) => (
          <article className="recent-item" key={folder.path}>
            <div className="folder-icon" aria-hidden="true">
              <Folder />
            </div>
            <div className="folder-copy">
              <h3>{folder.path}</h3>
              <p>
                {folder.documents} · Última utilização: {folder.usedAt}
              </p>
            </div>
            <button className="button button-secondary compact" type="button">
              Abrir
            </button>
          </article>
        ))}
      </div>
    </section>
  );
}

function InfoPanel() {
  return (
    <aside className="info-panel" aria-label="Benefícios">
      {benefits.map((benefit) => (
        <article className="benefit" key={benefit.title}>
          <h2>{benefit.title}</h2>
          <p>{benefit.text}</p>
        </article>
      ))}
    </aside>
  );
}

function StatusBar() {
  return (
    <footer className="status-bar" aria-label="Status">
      <span>0 documentos</span>
      <span>Indexed</span>
      <span>Local-first</span>
    </footer>
  );
}

export default App;
