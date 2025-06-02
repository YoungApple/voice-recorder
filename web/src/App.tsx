import React from "react";
import { BrowserRouter as Router, Routes, Route } from "react-router-dom";
import Home from "./pages/Home";
import Detail from "./pages/Detail";
import Ideas from "./pages/Ideas";
import Tasks from "./pages/Tasks";

/**
 * Main App component that handles routing for the voice recorder application
 * Features:
 * - Home page with recording functionality and notes list
 * - Detail page for viewing individual recordings
 * - Modern UI with Tailwind CSS
 */
function App() {
  return (
    <Router>
      <div className="min-h-screen bg-gradient-to-br from-pink-100 to-blue-100">
        <Routes>
          <Route path="/" element={<Home />} />
          <Route path="/detail/:id" element={<Detail />} />
          <Route path="/ideas" element={<Ideas />} />
          <Route path="/tasks" element={<Tasks />} />
          {/* Fallback route */}
          <Route path="*" element={<Home />} />
        </Routes>
      </div>
    </Router>
  );
}

export default App;