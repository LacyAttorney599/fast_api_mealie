import { NavLink, useLocation } from "react-router-dom";
import styles from "./Sidebar.module.css";

const navItems = [
  {
    to: "/",
    label: "Recettes",
    icon: (
      <path d="M4 19.5A2.5 2.5 0 0 1 6.5 17H20M6.5 2H20v20H6.5A2.5 2.5 0 0 1 4 19.5v-15A2.5 2.5 0 0 1 6.5 2z" />
    ),
    isActive: (pathname: string) => pathname === "/" || pathname.startsWith("/recette"),
  },
  {
    to: "/planificateur",
    label: "Planificateur",
    icon: (
      <>
        <rect x="3" y="4" width="18" height="18" rx="2" />
        <path d="M16 2v4M8 2v4M3 10h18" />
      </>
    ),
    isActive: (pathname: string) => pathname.startsWith("/planificateur"),
  },
  {
    to: "/courses",
    label: "Liste de courses",
    icon: (
      <>
        <circle cx="9" cy="21" r="1" />
        <circle cx="20" cy="21" r="1" />
        <path d="M1 1h4l2.68 13.39a2 2 0 0 0 2 1.61h9.72a2 2 0 0 0 2-1.61L23 6H6" />
      </>
    ),
    isActive: (pathname: string) => pathname.startsWith("/courses"),
  },
  {
    to: "/importer",
    label: "Importer",
    icon: (
      <>
        <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
        <path d="M17 8l-5-5-5 5M12 3v12" />
      </>
    ),
    isActive: (pathname: string) => pathname.startsWith("/importer"),
  },
];

export default function Sidebar() {
  const { pathname } = useLocation();

  return (
    <div className={styles.sidebar}>
      <div className={styles.brand}>
        <div className={styles.logo}>
          <svg width="19" height="19" viewBox="0 0 24 24" fill="none" stroke="#FAF7F2" strokeWidth={2} strokeLinecap="round" strokeLinejoin="round">
            <path d="M18 2v6a4 4 0 0 1-4 4H6" />
            <path d="M6 2v20" />
            <path d="M2 2v6a4 4 0 0 0 4 4" />
          </svg>
        </div>
        <span className={styles.brandName}>Ma Cuisine</span>
      </div>

      <nav className={styles.nav}>
        {navItems.map((item) => (
          <NavLink
            key={item.to}
            to={item.to}
            className={`${styles.navLink} ${item.isActive(pathname) ? styles.navLinkActive : ""}`}
          >
            <svg width="19" height="19" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={2} strokeLinecap="round" strokeLinejoin="round">
              {item.icon}
            </svg>
            {item.label}
          </NavLink>
        ))}
      </nav>

      <div className={styles.footer}>
        <NavLink to="/ajouter" className={styles.newRecipeButton}>
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="#FFFFFF" strokeWidth={2.5} strokeLinecap="round">
            <path d="M12 5v14M5 12h14" />
          </svg>
          Nouvelle recette
        </NavLink>
      </div>
    </div>
  );
}
