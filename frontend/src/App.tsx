import { Routes, Route } from "react-router-dom";
import Layout from "./components/Layout";
import Liste from "./pages/Liste";
import Detail from "./pages/Detail";
import Ajouter from "./pages/Ajouter";
import Importer from "./pages/Importer";
import Planificateur from "./pages/Planificateur";
import Courses from "./pages/Courses";

export default function App() {
  return (
    <Routes>
      <Route element={<Layout />}>
        <Route index element={<Liste />} />
        <Route path="recette/:slug" element={<Detail />} />
        <Route path="recette/:slug/modifier" element={<Ajouter />} />
        <Route path="ajouter" element={<Ajouter />} />
        <Route path="importer" element={<Importer />} />
        <Route path="planificateur" element={<Planificateur />} />
        <Route path="courses" element={<Courses />} />
      </Route>
    </Routes>
  );
}
