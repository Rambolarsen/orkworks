import ReactDOM from "react-dom/client";
import App from "./App";
import "./styles/tokens.css";
import "./App.css";
import "dockview-react/dist/styles/dockview.css";

document.documentElement.dataset.platform = window.orkworks.platform;
ReactDOM.createRoot(document.getElementById("root")!).render(<App />);
