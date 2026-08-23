import { render, screen } from "@testing-library/react";
import App from "./App";

describe("FlokinMD shell", () => {
  it("renders the desktop shell content for MDB-001", () => {
    render(<App />);

    expect(screen.getByLabelText("FlokinMD")).toBeInTheDocument();
    expect(
      screen.getByRole("heading", {
        name: /transforme seus arquivos markdown/i,
      }),
    ).toBeInTheDocument();
    expect(screen.getByPlaceholderText("Buscar documentos...")).toBeInTheDocument();
    expect(screen.getByText("Pastas recentes")).toBeInTheDocument();
    expect(screen.getByText("~/Documents/Knowledge")).toBeInTheDocument();
    expect(screen.getByText("0 documentos")).toBeInTheDocument();
  });
});
